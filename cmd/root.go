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
		Version:       Version,
	}

	var wdir string
	root.PersistentFlags().StringVarP(&wdir, "workspace", "w", ".", "workspace directory")

	// Define groups
	resourceGroup := &cobra.Group{
		ID:    "resource",
		Title: "CRUD actions on resources:",
	}
	secondaryGroup := &cobra.Group{
		ID:    "secondary",
		Title: "Secondary actions:",
	}
	root.AddGroup(resourceGroup, secondaryGroup)

	// CRUD Commands
	createCmd := newCreateCmd(&wdir)
	createCmd.GroupID = resourceGroup.ID

	updateCmd := newUpdateCmd(&wdir)
	updateCmd.GroupID = resourceGroup.ID

	connectCmd := newConnectCmd(&wdir)
	connectCmd.GroupID = resourceGroup.ID

	removeCmd := newRemoveCmd(&wdir)
	removeCmd.GroupID = resourceGroup.ID

	// Secondary Commands
	initCmd := newInitCmd()
	initCmd.GroupID = secondaryGroup.ID

	loginCmd := newLoginCmd(&wdir)
	loginCmd.GroupID = secondaryGroup.ID

	validateCmd := newValidateCmd(&wdir)
	validateCmd.GroupID = secondaryGroup.ID

	planCmd := newPlanCmd(&wdir)
	planCmd.GroupID = secondaryGroup.ID

	applyCmd := newApplyCmd(&wdir)
	applyCmd.GroupID = secondaryGroup.ID

	exportCmd := newExportCmd(&wdir)
	exportCmd.GroupID = secondaryGroup.ID

	pullCmd := newPullCmd(&wdir)
	pullCmd.GroupID = secondaryGroup.ID

	statusCmd := newStatusCmd(&wdir)
	statusCmd.GroupID = secondaryGroup.ID

	versionCmd := newVersionCmd()
	versionCmd.GroupID = secondaryGroup.ID

	root.AddCommand(
		initCmd,
		loginCmd,
		validateCmd,
		planCmd,
		applyCmd,
		exportCmd,
		pullCmd,
		statusCmd,
		createCmd,
		updateCmd,
		connectCmd,
		removeCmd,
		versionCmd,
	)

	// Add completion and help explicitly to set their GroupID
	root.InitDefaultHelpCmd()
	root.InitDefaultCompletionCmd()

	for _, cmd := range root.Commands() {
		if cmd.Name() == "completion" || cmd.Name() == "help" {
			cmd.GroupID = secondaryGroup.ID
		}
	}

	return root
}
