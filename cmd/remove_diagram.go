package cmd

import (
	"fmt"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

// apiKeyFromWorkspace returns the API key from workspace config (which already
// has the TLD_API_KEY env var merged in by workspace.Load).
func apiKeyFromWorkspace(cfg workspace.Config) string {
	return cfg.APIKey
}

func newRemoveDiagramCmd(wdir *string) *cobra.Command {
	var offline bool

	cmd := &cobra.Command{
		Use:   "diagram <ref>",
		Short: "Remove a diagram and cascade-delete its edges, links, and object placements",
		Args:  cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			ref := args[0]

			// Load workspace before deletion so we can read meta IDs and config.
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			// Capture the server ID before the local YAML is modified.
			var serverID workspace.ResourceID
			if ws.Meta != nil {
				if m, ok := ws.Meta.Diagrams[ref]; ok {
					serverID = m.ID
				}
			}

			// Local YAML cleanup (always runs first).
			edges, links, placements, err := workspace.DeleteDiagram(*wdir, ref)
			if err != nil {
				return fmt.Errorf("delete diagram: %w", err)
			}
			fmt.Fprintf(cmd.OutOrStdout(), "Removed %s from diagrams.yaml\n", ref)
			if edges > 0 {
				fmt.Fprintf(cmd.OutOrStdout(), "  cascade: removed %d edge(s) in that diagram\n", edges)
			}
			if links > 0 {
				fmt.Fprintf(cmd.OutOrStdout(), "  cascade: removed %d link(s) referencing that diagram\n", links)
			}
			if placements > 0 {
				fmt.Fprintf(cmd.OutOrStdout(), "  cascade: removed %d placement(s) referencing that diagram\n", placements)
			}

			// Server deletion - skipped when --offline or no server ID is available.
			if offline || serverID == 0 || ws.Config.ServerURL == "" {
				if !offline && serverID == 0 {
					fmt.Fprintln(cmd.OutOrStdout(), "  hint: no server ID found; run 'tld apply' to sync with the server")
				}
				return nil
			}

			apiKey := apiKeyFromWorkspace(ws.Config)
			if apiKey == "" {
				fmt.Fprintln(cmd.ErrOrStderr(), "warning: TLD_API_KEY not set and no api_key in .tld.yaml; skipping server delete")
				return nil
			}

			c := client.New(ws.Config.ServerURL, apiKey, false)
			req := connect.NewRequest(&diagv1.DeleteViewRequest{
				OrgId:  ws.Config.OrgID,
				ViewId: int32(serverID),
			})

			_, err = c.DeleteView(cmd.Context(), req)
			if err != nil {
				if connectErr, ok := err.(*connect.Error); ok && connectErr.Code() == connect.CodeNotFound {
					fmt.Fprintln(cmd.OutOrStdout(), "  server: diagram not found (already deleted or never applied)")
				} else {
					fmt.Fprintf(cmd.ErrOrStderr(), "warning: server delete failed: %v\n", err)
					fmt.Fprintln(cmd.OutOrStdout(), "  local YAML updated. Run 'tld apply' to retry sync.")
				}
				return nil
			}

			fmt.Fprintf(cmd.OutOrStdout(), "  deleted from server (id=%d)\n", serverID)
			return nil
		},
	}

	cmd.Flags().BoolVar(&offline, "offline", false, "skip server deletion, only update local YAML")
	return cmd
}
