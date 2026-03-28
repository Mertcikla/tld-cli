package cmd

import (
	"fmt"
	"time"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newStatusCmd(wdir *string) *cobra.Command {
	return &cobra.Command{
		Use:   "status",
		Short: "Show sync status between local YAML and the server",
		Long: `Status compares the local workspace against the last known sync point.

It reports whether your local YAML files have changed since the last
'tld apply' or 'tld pull', helping you decide whether to push or pull.`,
		RunE: func(cmd *cobra.Command, _ []string) error {
			lockFile, err := workspace.LoadLockFile(*wdir)
			if err != nil {
				return fmt.Errorf("load lock file: %w", err)
			}
			if lockFile == nil {
				fmt.Fprintln(cmd.OutOrStdout(), "No sync history found. Run 'tld pull' or 'tld apply' first.")
				return nil
			}

			currentHash, err := workspace.CalculateWorkspaceHash(*wdir)
			if err != nil {
				return fmt.Errorf("calculate hash: %w", err)
			}

			fmt.Fprintf(cmd.OutOrStdout(), "Last sync:     %s\n", lockFile.LastApply.Format(time.RFC3339))
			fmt.Fprintf(cmd.OutOrStdout(), "Applied by:    %s\n", lockFile.AppliedBy)
			fmt.Fprintf(cmd.OutOrStdout(), "Version:       %s\n", lockFile.VersionID)

			if lockFile.Resources != nil {
				fmt.Fprintf(cmd.OutOrStdout(), "Resources:     %d diagrams, %d objects, %d edges, %d links\n",
					lockFile.Resources.Diagrams, lockFile.Resources.Objects,
					lockFile.Resources.Edges, lockFile.Resources.Links)
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "Resources:     unknown")
			}

			if lockFile.WorkspaceHash == "" || currentHash == lockFile.WorkspaceHash {
				fmt.Fprintln(cmd.OutOrStdout(), "Local changes: Clean")
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "Local changes: Modified (run 'tld apply' to push or 'tld pull --force' to reset)")
			}

			return nil
		},
	}
}
