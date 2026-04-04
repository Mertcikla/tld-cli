package cmd

import (
	"fmt"
	"os"
	"os/exec"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newDiffCmd(wdir *string) *cobra.Command {
	c := &cobra.Command{
		Use:   "diff",
		Short: "Show differences between local workspace and server",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			targetOrg := ws.Config.OrgID
			if targetOrg == "" {
				return fmt.Errorf("org-id required in .tld.yaml")
			}

			// 1. Export server state to a temp directory
			tempDir, err := os.MkdirTemp("", "tld-diff-*")
			if err != nil {
				return fmt.Errorf("create temp dir: %w", err)
			}
			defer func() { _ = os.RemoveAll(tempDir) }()

			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
			resp, err := c.ExportOrganization(cmd.Context(), connect.NewRequest(&diagv1.ExportOrganizationRequest{
				OrgId: targetOrg,
			}))
			if err != nil {
				return fmt.Errorf("fetch server state: %w", err)
			}

			serverWS := convertExportResponse(&workspace.Workspace{Dir: tempDir, Config: ws.Config}, resp.Msg)
			if err := workspace.Save(serverWS); err != nil {
				return fmt.Errorf("save server state to temp: %w", err)
			}

			// 2. Run git diff between temp and local
			// Note: we use --no-index to diff directories not in a git repo
			// We diff FROM server TO local (so + means local addition, - means server has it but local doesn't)
			diffCmd := exec.Command("git", "diff", "--no-index", "--color=always", tempDir, *wdir)
			diffCmd.Stdout = cmd.OutOrStdout()
			diffCmd.Stderr = cmd.ErrOrStderr()

			// git diff --no-index returns 1 if differences are found, which is fine
			_ = diffCmd.Run()

			return nil
		},
	}

	return c
}
