package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// Version is the current version of the CLI.
// This is overridden by ldflags during build.
var Version = "0.0.0-dev"

func newVersionCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "version",
		Short: "Print the version number of tld",
		Run: func(cmd *cobra.Command, _ []string) {
			fmt.Fprintf(cmd.OutOrStdout(), "tld version %s\n", Version)
		},
	}
}
