package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var rootCmd = NewRootCmd()

// Execute runs the root command and exits on error.
func Execute() {
	if err := rootCmd.Execute(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}

// NewRootCmd creates a fresh, isolated root command. Called by Execute() for the
// binary and by tests to get a clean instance with no shared state.
func NewRootCmd() *cobra.Command {
	root := &cobra.Command{
		Use:   "tld",
		Short: "tld -- tlDiagram CLI",
		Long: `tld manages software architecture diagrams as code.

Define your architecture in YAML, preview changes with 'tld plan',
and apply them atomically with 'tld apply'.`,
		SilenceErrors: true,
		SilenceUsage:  true,
	}

	var wdir string
	root.PersistentFlags().StringVarP(&wdir, "workspace", "w", ".", "workspace directory")

	root.AddCommand(
		newInitCmd(),
		newLoginCmd(&wdir),
		newValidateCmd(&wdir),
		newPlanCmd(&wdir),
		newApplyCmd(&wdir),
		newExportCmd(&wdir),
		newPullCmd(&wdir),
		newStatusCmd(&wdir),
		newCreateCmd(&wdir),
		newConnectCmd(&wdir),
		newAddCmd(&wdir),
		newRemoveCmd(&wdir),
	)

	return root
}
