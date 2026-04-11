package cmd

import (
	"bufio"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
	"gopkg.in/yaml.v3"
)

var defaultWorkspaceExclude = []string{
	"vendor/",
	"node_modules/",
	".git/",
	"**/*_test.go",
	"**/*.pb.go",
}

const defaultWorkspaceConfig = `# tld workspace configuration
# project metadata and repo-scoped analysis settings for tld analyze and tld check
project_name: ""
exclude:
- vendor/
- node_modules/
- .git/
- "**/*_test.go"
- "**/*.pb.go"
repositories: {}
# Example:
# frontend:
#   url: github.com/example/frontend
#   localDir: frontend
#   root: bKLqGV48
#   config:
#     mode: auto
#   exclude:
#     - generated/**
`

func newInitCmd() *cobra.Command {
	var wizard bool
	cmd := &cobra.Command{
		Use:   "init [dir]",
		Short: "Initialize a new tld workspace",
		Args:  cobra.MaximumNArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			dir := "tld"
			if len(args) > 0 {
				dir = args[0]
			}

			if err := os.MkdirAll(dir, 0750); err != nil {
				return fmt.Errorf("create %s: %w", dir, err)
			}

			// Create empty YAML files if they don't exist
			files := map[string]string{
				"diagrams.yaml":   "{}\n",
				"objects.yaml":    "{}\n",
				"edges.yaml":      "{}\n",
				"links.yaml":      "[]\n",
				"elements.yaml":   "{}\n",
				"connectors.yaml": "{}\n",
			}
			for f, content := range files {
				path := filepath.Join(dir, f)
				if _, err := os.Stat(path); os.IsNotExist(err) {
					if err := os.WriteFile(path, []byte(content), 0600); err != nil {
						return fmt.Errorf("create %s: %w", f, err)
					}
				}
			}

			workspaceConfigPath := filepath.Join(dir, ".tld.yaml")
			if wizard {
				if err := runInitWizard(cmd, dir); err != nil {
					return err
				}
			} else if _, err := os.Stat(workspaceConfigPath); os.IsNotExist(err) {
				if err := os.WriteFile(workspaceConfigPath, []byte(defaultWorkspaceConfig), 0600); err != nil {
					return fmt.Errorf("create .tld.yaml: %w", err)
				}
			} else if err != nil {
				return fmt.Errorf("stat .tld.yaml: %w", err)
			}

			cfgPath, err := workspace.ConfigPath()
			if err != nil {
				return fmt.Errorf("get config path: %w", err)
			}

			if err := os.MkdirAll(filepath.Dir(cfgPath), 0700); err != nil {
				return fmt.Errorf("create config dir: %w", err)
			}

			if _, err := os.Stat(cfgPath); err == nil {
				fmt.Fprintf(cmd.OutOrStdout(), "Initialized workspace at %s (global config already exists at %s)\n", dir, cfgPath)
			} else {
				cfg := `# tld global configuration
server_url: https://tldiagram.com
api_key: ""        # or set TLD_API_KEY env var
org_id: ""         # UUID of your organisation
`
				if err := os.WriteFile(cfgPath, []byte(cfg), 0600); err != nil {
					return fmt.Errorf("write tld.yaml: %w", err)
				}
				fmt.Fprintf(cmd.OutOrStdout(), "Initialized workspace at %s\n", dir)
				fmt.Fprintf(cmd.OutOrStdout(), "Global configuration file created: %s\n", cfgPath)
			}

			if !wizard {
				fmt.Fprintln(cmd.OutOrStdout(), "Run `tld login` to authenticate with tldiagram.com")
			}
			return nil
		},
	}
	cmd.Flags().BoolVar(&wizard, "wizard", false, "run interactive setup wizard")
	return cmd
}

func runInitWizard(cmd *cobra.Command, dir string) error {
	scanner := bufio.NewScanner(cmd.InOrStdin())
	projectName, err := promptRequired(scanner, cmd.OutOrStdout(), "Project name")
	if err != nil {
		return err
	}

	repositories := make(map[string]workspace.Repository)
	for {
		repoKey, err := promptRequired(scanner, cmd.OutOrStdout(), "Repository key")
		if err != nil {
			return err
		}
		url, err := promptRequired(scanner, cmd.OutOrStdout(), "Repository URL")
		if err != nil {
			return err
		}
		localDir, err := promptRequired(scanner, cmd.OutOrStdout(), "Repository local dir")
		if err != nil {
			return err
		}
		mode, err := promptRepositoryMode(scanner, cmd.OutOrStdout())
		if err != nil {
			return err
		}

		repositories[repoKey] = workspace.Repository{
			URL:      url,
			LocalDir: localDir,
			Config: &workspace.RepositoryConfig{
				Mode: mode,
			},
		}

		addAnother, err := promptYesNo(scanner, cmd.OutOrStdout(), "Add another repository? [y/N]", false)
		if err != nil {
			return err
		}
		if !addAnother {
			break
		}
	}

	config := workspace.WorkspaceConfig{
		ProjectName:  projectName,
		Exclude:      append([]string{}, defaultWorkspaceExclude...),
		Repositories: repositories,
	}
	data, err := yaml.Marshal(&config)
	if err != nil {
		return fmt.Errorf("marshal .tld.yaml: %w", err)
	}
	if err := os.WriteFile(filepath.Join(dir, ".tld.yaml"), data, 0600); err != nil {
		return fmt.Errorf("write .tld.yaml: %w", err)
	}

	fmt.Fprintln(cmd.OutOrStdout(), "Next steps:")
	fmt.Fprintln(cmd.OutOrStdout(), "  1. tld login          - authenticate with tlDiagram.com")
	fmt.Fprintln(cmd.OutOrStdout(), "  2. tld analyze .      - extract symbols from your repo")
	fmt.Fprintln(cmd.OutOrStdout(), "  3. tld plan           - preview what will be created")
	fmt.Fprintln(cmd.OutOrStdout(), "  4. tld apply          - push to tlDiagram.com")
	return nil
}

func promptRequired(scanner *bufio.Scanner, out interface{ Write([]byte) (int, error) }, label string) (string, error) {
	for {
		fmt.Fprintf(out, "%s: ", label)
		if !scanner.Scan() {
			if err := scanner.Err(); err != nil {
				return "", err
			}
			return "", fmt.Errorf("%s is required", strings.ToLower(label))
		}
		value := strings.TrimSpace(scanner.Text())
		if value != "" {
			return value, nil
		}
	}
}

func promptRepositoryMode(scanner *bufio.Scanner, out interface{ Write([]byte) (int, error) }) (string, error) {
	fmt.Fprintln(out, "Select repository mode:")
	fmt.Fprintln(out, "  1. upsert - add new symbols, update existing - never delete (recommended)")
	fmt.Fprintln(out, "  2. manual - no automatic changes - full manual control")
	fmt.Fprintln(out, "  3. auto   - create, update, and delete based on source analysis")

	for {
		fmt.Fprint(out, "Mode [1]: ")
		if !scanner.Scan() {
			if err := scanner.Err(); err != nil {
				return "", err
			}
			return "", fmt.Errorf("repository mode is required")
		}
		switch strings.ToLower(strings.TrimSpace(scanner.Text())) {
		case "", "1", "upsert":
			return "upsert", nil
		case "2", "manual":
			return "manual", nil
		case "3", "auto":
			return "auto", nil
		}
	}
}

func promptYesNo(scanner *bufio.Scanner, out interface{ Write([]byte) (int, error) }, label string, defaultYes bool) (bool, error) {
	for {
		fmt.Fprintf(out, "%s ", label)
		if !scanner.Scan() {
			if err := scanner.Err(); err != nil {
				return false, err
			}
			return defaultYes, nil
		}
		switch strings.ToLower(strings.TrimSpace(scanner.Text())) {
		case "":
			return defaultYes, nil
		case "y", "yes":
			return true, nil
		case "n", "no":
			return false, nil
		}
	}
}
