package cmd

import (
	"fmt"
	"time"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newExportCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "export [org-id]",
		Short: "Export all diagrams from an organization to the local workspace",
		Args:  cobra.MaximumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w (did you run 'tld init'?)", err)
			}

			targetOrg := ws.Config.OrgID
			if len(args) > 0 {
				targetOrg = args[0]
			}
			if targetOrg == "" {
				return fmt.Errorf("org-id required (either as argument or in .tld.yaml)")
			}

			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
			resp, err := c.ExportWorkspace(cmd.Context(), connect.NewRequest(&diagv1.ExportOrganizationRequest{
				OrgId: targetOrg,
			}))
			if err != nil {
				return fmt.Errorf("export failed: %w", err)
			}

			newWS := convertExportResponse(ws, resp.Msg)

			if err := workspace.Save(newWS); err != nil {
				return fmt.Errorf("save workspace: %w", err)
			}

			// Update lock file so version tracking stays consistent
			hash, err := workspace.CalculateWorkspaceHash(*wdir)
			if err != nil {
				return fmt.Errorf("calculate hash: %w", err)
			}
			lockFile, err := workspace.LoadLockFile(*wdir)
			if err != nil {
				return fmt.Errorf("load lock file: %w", err)
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

			fmt.Fprintf(cmd.OutOrStdout(), "Exported %d diagrams, %d objects, %d edges, %d links to %s\n",
				len(newWS.Diagrams), len(newWS.Objects), len(newWS.Edges), len(newWS.Links), *wdir)

			return nil
		},
	}

	return c
}
