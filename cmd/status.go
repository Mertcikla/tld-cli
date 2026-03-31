package cmd

import (
	"fmt"
	"time"

	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
	"google.golang.org/protobuf/proto"
)

func newStatusCmd(wdir *string) *cobra.Command {
	var checkServer bool

	c := &cobra.Command{
		Use:   "status",
		Short: "Show sync status between local YAML and the server",
		Long: `Status compares the local workspace against the last known sync point.

With --check-server, it also performs a dry-run on the server to detect
any drift from manual changes in the frontend.`,
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}
			lockFile, err := workspace.LoadLockFile(*wdir)
			if err != nil {
				return fmt.Errorf("load lock file: %w", err)
			}

			if lockFile != nil {
				fmt.Fprintf(cmd.OutOrStdout(), "Last sync:     %s\n", lockFile.LastApply.Format(time.RFC3339))
				fmt.Fprintf(cmd.OutOrStdout(), "Applied by:    %s\n", lockFile.AppliedBy)
				fmt.Fprintf(cmd.OutOrStdout(), "Version:       %s\n", lockFile.VersionID)

				currentHash, err := workspace.CalculateWorkspaceHash(*wdir)
				if err == nil {
					if lockFile.WorkspaceHash == "" || currentHash == lockFile.WorkspaceHash {
						fmt.Fprintln(cmd.OutOrStdout(), "Local changes: Clean")
					} else {
						fmt.Fprintln(cmd.OutOrStdout(), "Local changes: Modified")
					}
				}
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "No sync history found.")
			}

			if checkServer {
				fmt.Fprintln(cmd.OutOrStdout(), "\nChecking server drift...")
				c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
				plan, err := planner.Build(ws, false)
				if err != nil {
					return fmt.Errorf("build plan: %w", err)
				}
				plan.Request.DryRun = proto.Bool(true)
				resp, err := c.ApplyPlan(cmd.Context(), connect.NewRequest(plan.Request))
				if err != nil {
					return fmt.Errorf("server check failed: %w", err)
				}

				if len(resp.Msg.Drift) == 0 && len(resp.Msg.Conflicts) == 0 {
					fmt.Fprintln(cmd.OutOrStdout(), "Server state:  In sync")
				} else {
					if len(resp.Msg.Drift) > 0 {
						fmt.Fprintf(cmd.OutOrStdout(), "Server state:  %d drift items found (run 'tld pull' to sync)\n", len(resp.Msg.Drift))
						for _, d := range resp.Msg.Drift {
							fmt.Fprintf(cmd.OutOrStdout(), "  - %s: %s (%s)\n", d.ResourceType, d.Ref, d.Reason)
						}
					}
					if len(resp.Msg.Conflicts) > 0 {
						fmt.Fprintf(cmd.OutOrStdout(), "Server state:  %d conflicts found (run 'tld pull' or 'tld apply' to resolve)\n", len(resp.Msg.Conflicts))
					}
				}
			}

			return nil
		},
	}

	c.Flags().BoolVar(&checkServer, "check-server", false, "check against the live server state")
	return c
}
