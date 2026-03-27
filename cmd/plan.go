package cmd

import (
	"fmt"
	"os"
	"time"

	"connectrpc.com/connect"
	"github.com/mertcikla/tldiagram-cli/client"
	"github.com/mertcikla/tldiagram-cli/planner"
	"github.com/mertcikla/tldiagram-cli/workspace"
	"github.com/spf13/cobra"
	"google.golang.org/protobuf/proto"
)

func newPlanCmd(wdir *string) *cobra.Command {
	var planOutput string
	var recreateIDs bool

	c := &cobra.Command{
		Use:   "plan",
		Short: "Show what would be applied",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}
			if errs := ws.Validate(); len(errs) > 0 {
				for _, e := range errs {
					fmt.Fprintf(cmd.ErrOrStderr(), "validation error: %s\n", e)
				}
				return fmt.Errorf("workspace has %d validation error(s)", len(errs))
			}
			plan, err := planner.Build(ws, recreateIDs)
			if err != nil {
				return fmt.Errorf("build plan: %w", err)
			}

			// Perform dry run on the server to detect conflicts and drift
			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
			req := plan.Request
			req.DryRun = proto.Bool(true)

			resp, err := c.ApplyPlan(cmd.Context(), connect.NewRequest(req))
			if err != nil {
				return fmt.Errorf("server plan failed: %w", err)
			}

			out := cmd.OutOrStdout()
			if planOutput != "" {
				f, err := os.Create(planOutput)
				if err != nil {
					return fmt.Errorf("create output file: %w", err)
				}
				defer func() { _ = f.Close() }()
				out = f
			}

			planner.RenderPlanMarkdown(out, plan, ws)

			// Show conflicts and drift if any
			if len(resp.Msg.Conflicts) > 0 {
				fmt.Fprintf(cmd.OutOrStdout(), "\n⚠️  %d conflicts detected:\n", len(resp.Msg.Conflicts))
				for _, c := range resp.Msg.Conflicts {
					fmt.Fprintf(cmd.OutOrStdout(), "  * %s \"%s\" (remote is newer: %s)\n",
						c.ResourceType, c.Ref, c.RemoteUpdatedAt.AsTime().Format(time.RFC3339))
				}
			}

			if len(resp.Msg.Drift) > 0 {
				fmt.Fprintf(cmd.OutOrStdout(), "\n🔍 %d drift items detected:\n", len(resp.Msg.Drift))
				for _, d := range resp.Msg.Drift {
					fmt.Fprintf(cmd.OutOrStdout(), "  * %s \"%s\": %s\n", d.ResourceType, d.Ref, d.Reason)
				}
			}

			return nil
		},
	}

	c.Flags().StringVarP(&planOutput, "output", "o", "", "write plan to file instead of stdout")
	c.Flags().BoolVar(&recreateIDs, "recreate-ids", false, "ignore existing resource IDs and let the server generate new ones")
	return c
}
