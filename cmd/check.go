package cmd

import (
	"context"
	"fmt"
	"os"

	"github.com/mertcikla/tld-cli/internal/git"
	"github.com/mertcikla/tld-cli/internal/symbol"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newCheckCmd(wdir *string) *cobra.Command {
	var strict bool

	c := &cobra.Command{
		Use:   "check",
		Short: "Check workspace validity for CI/CD pipelines",
		Long: `Validates the workspace YAML, verifies code symbols, and detects outdated diagrams.

Exit codes:
  0 — all checks passed
  1 — validation errors or broken symbols
  2 — outdated diagrams detected (only when --strict is set)`,
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			allPassed := true

			// ── 1. Schema validation ──────────────────────────────────────────────
			errs := ws.Validate()
			if len(errs) > 0 {
				fmt.Fprintln(cmd.ErrOrStderr(), "FAIL  Validation")
				for _, e := range errs {
					fmt.Fprintf(cmd.ErrOrStderr(), "      - %s\n", e)
				}
				allPassed = false
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "PASS  Validation")
			}

			// ── 2. Symbol verification ────────────────────────────────────────────
			broken := checkSymbols(cmd.Context(), ws)
			if len(broken) > 0 {
				fmt.Fprintln(cmd.ErrOrStderr(), "FAIL  Symbol Verification")
				for _, msg := range broken {
					fmt.Fprintf(cmd.ErrOrStderr(), "      - %s\n", msg)
				}
				allPassed = false
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "PASS  Symbol Verification")
			}

			// ── 3. Outdated diagram detection ─────────────────────────────────────
			outdated := checkOutdated(ws)
			if len(outdated) > 0 {
				label := "WARN "
				if strict {
					label = "FAIL "
					allPassed = false
				}
				fmt.Fprintf(cmd.ErrOrStderr(), "%s Outdated Diagrams\n", label)
				for _, msg := range outdated {
					fmt.Fprintf(cmd.ErrOrStderr(), "      - %s\n", msg)
				}
				if strict {
					fmt.Fprintln(cmd.ErrOrStderr(), "      (use `tld apply` to sync diagram metadata)")
				}
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "PASS  Outdated Diagrams")
			}

			if !allPassed {
				os.Exit(1)
			}
			return nil
		},
	}

	c.Flags().BoolVar(&strict, "strict", false, "exit non-zero when outdated diagrams are detected")
	return c
}

// checkSymbols verifies that elements with file_path+symbol have the symbol present
// in the referenced file.  Returns a list of human-readable failure messages.
func checkSymbols(ctx context.Context, ws *workspace.Workspace) []string {
	var failures []string
	for ref, element := range ws.Elements {
		if element.FilePath == "" || element.Symbol == "" {
			continue
		}
		if _, err := os.Stat(element.FilePath); err != nil {
			continue // file not accessible locally — skip
		}
		found, err := symbol.HasSymbol(ctx, element.FilePath, element.Symbol)
		if err != nil {
			if _, unsupported := err.(symbol.ErrUnsupportedLanguage); unsupported {
				continue
			}
			failures = append(failures, fmt.Sprintf("elements.yaml[%s]: %v", ref, err))
			continue
		}
		if !found {
			failures = append(failures, fmt.Sprintf(
				"elements.yaml[%s]: symbol %q not found in %s",
				ref, element.Symbol, element.FilePath,
			))
		}
	}
	return failures
}

// checkOutdated compares the last git commit timestamp for each element's file_path
// against the element's metadata UpdatedAt timestamp.  Returns human-readable messages
// for elements whose file was committed after the diagram was last synced.
func checkOutdated(ws *workspace.Workspace) []string {
	var outdated []string

	if ws.Meta == nil || ws.Meta.Elements == nil {
		return nil
	}

	// Find the git root from the workspace directory
	repoRoot, err := git.RepoRoot(ws.Dir)
	if err != nil {
		return nil // not a git repo — skip
	}

	for ref, element := range ws.Elements {
		if element.FilePath == "" {
			continue
		}
		meta, ok := ws.Meta.Elements[ref]
		if !ok || meta.UpdatedAt.IsZero() {
			continue
		}
		commitTime, err := git.FileLastCommitAt(repoRoot, element.FilePath)
		if err != nil {
			continue // file not tracked — skip
		}
		if commitTime.After(meta.UpdatedAt) {
			outdated = append(outdated, fmt.Sprintf(
				"elements.yaml[%s]: file %s changed %s, diagram last synced %s",
				ref,
				element.FilePath,
				commitTime.Format("2006-01-02 15:04:05"),
				meta.UpdatedAt.Format("2006-01-02 15:04:05"),
			))
		}
	}
	return outdated
}
