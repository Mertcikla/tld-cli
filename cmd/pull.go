package cmd

import (
	"bufio"
	"errors"
	"fmt"
	"strings"
	"time"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newPullCmd(wdir *string) *cobra.Command {
	var force bool
	var dryRun bool

	c := &cobra.Command{
		Use:   "pull",
		Short: "Pull the current server state into local YAML files",
		Long: `Pull downloads the current diagram state from the server and overwrites
local YAML files. Use this after making changes in the frontend UI.

If you have local changes that haven't been applied yet, tld pull will warn
you before overwriting them. Use --force to skip the prompt.`,
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w (did you run 'tld init'?)", err)
			}

			targetOrg := ws.Config.OrgID
			if targetOrg == "" {
				return fmt.Errorf("org-id required in .tld.yaml")
			}

			lockFile, err := workspace.LoadLockFile(*wdir)
			if err != nil {
				return fmt.Errorf("load lock file: %w", err)
			}

			// Detect local changes if we have a previous hash to compare against
			if !force && lockFile != nil && lockFile.WorkspaceHash != "" {
				currentHash, err := workspace.CalculateWorkspaceHash(*wdir)
				if err != nil {
					return fmt.Errorf("calculate hash: %w", err)
				}
				if currentHash != lockFile.WorkspaceHash {
					fmt.Fprintf(cmd.OutOrStdout(), "Warning: local workspace has uncommitted changes.\n")
					fmt.Fprintf(cmd.OutOrStdout(), "Pull will overwrite them. Continue? [yes/no]: ")
					scanner := bufio.NewScanner(cmd.InOrStdin())
					if !scanner.Scan() {
						return errors.New("aborted")
					}
					answer := strings.TrimSpace(strings.ToLower(scanner.Text()))
					if answer != "yes" && answer != "y" {
						fmt.Fprintln(cmd.OutOrStdout(), "Pull cancelled.")
						return nil
					}
				}
			}

			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
			resp, err := c.ExportWorkspace(cmd.Context(), connect.NewRequest(&diagv1.ExportOrganizationRequest{
				OrgId: targetOrg,
			}))
			if err != nil {
				return fmt.Errorf("pull failed: %w", err)
			}

			newWS := convertExportResponse(ws, resp.Msg)

			if dryRun {
				fmt.Fprintf(cmd.OutOrStdout(), "Would pull: %d diagrams, %d objects, %d edges, %d links\n",
					len(newWS.Diagrams), len(newWS.Objects), len(newWS.Edges), len(newWS.Links))
				return nil
			}

			// Perform surgical merge
			lastSyncMeta := &workspace.Meta{
				Diagrams: make(map[string]*workspace.ResourceMetadata),
				Objects:  make(map[string]*workspace.ResourceMetadata),
				Edges:    make(map[string]*workspace.ResourceMetadata),
			}
			if lockFile != nil && lockFile.Metadata != nil {
				lastSyncMeta = lockFile.Metadata
			}

			if force {
				if err := workspace.Save(newWS); err != nil {
					return fmt.Errorf("force save workspace: %w", err)
				}
			} else {
				if err := workspace.MergeWorkspace(*wdir, newWS, lastSyncMeta, ws.Meta); err != nil {
					return fmt.Errorf("merge workspace: %w", err)
				}
			}

			hash, err := workspace.CalculateWorkspaceHash(*wdir)
			if err != nil {
				return fmt.Errorf("calculate hash: %w", err)
			}
			if lockFile == nil {
				lockFile = &workspace.LockFile{Version: "v1"}
			}
			versionID := fmt.Sprintf("pull-%s", time.Now().UTC().Format(time.RFC3339))
			workspace.UpdateLockFile(lockFile, versionID, "pull",
				len(newWS.Diagrams), len(newWS.Objects), len(newWS.Edges), len(newWS.Links),
				hash, nil, newWS.Meta)
			if err := workspace.WriteLockFile(*wdir, lockFile); err != nil {
				return fmt.Errorf("write lock file: %w", err)
			}

			fmt.Fprintf(cmd.OutOrStdout(), "Pulled %d diagrams, %d objects, %d edges, %d links\n",
				len(newWS.Diagrams), len(newWS.Objects), len(newWS.Edges), len(newWS.Links))

			return nil
		},
	}

	c.Flags().BoolVar(&force, "force", false, "overwrite local changes without prompting")
	c.Flags().BoolVar(&dryRun, "dry-run", false, "show what would be pulled without writing")
	return c
}
