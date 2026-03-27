package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newInitCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "init [dir]",
		Short: "Initialize a new tld workspace",
		Args:  cobra.MaximumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			dir := "."
			if len(args) > 0 {
				dir = args[0]
			}

			if err := os.MkdirAll(dir, 0750); err != nil {
				return fmt.Errorf("create %s: %w", dir, err)
			}

			cfgPath, err := workspace.ConfigPath()
			if err != nil {
				return fmt.Errorf("get config path: %w", err)
			}

			if err := os.MkdirAll(filepath.Dir(cfgPath), 0700); err != nil {
				return fmt.Errorf("create config dir: %w", err)
			}

			if _, err := os.Stat(cfgPath); err == nil {
				fmt.Fprintf(cmd.OutOrStdout(), "Initialized workspace at %s (config already exists at %s)\n", dir, cfgPath)
				return nil
			}

			cfg := `# tld workspace configuration
server_url: https://tldiagram.com
api_key: ""        # or set TLD_API_KEY env var
org_id: ""         # UUID of your organisation
`
			if err := os.WriteFile(cfgPath, []byte(cfg), 0600); err != nil {
				return fmt.Errorf("write tld.yaml: %w", err)
			}

			fmt.Fprintf(cmd.OutOrStdout(), "Initialized workspace at %s\n", dir)
			fmt.Fprintf(cmd.OutOrStdout(), "Configuration file: %s \n", cfgPath)
			fmt.Printf("Run `tld login` to authenticate with tldiagram.com \n")
			return nil
		},
	}
}
