package cmd

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/mertcikla/tld-cli/internal/git"
	"github.com/mertcikla/tld-cli/internal/ignore"
	"github.com/mertcikla/tld-cli/internal/symbol"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
)

func newAnalyzeCmd(wdir *string) *cobra.Command {
	var deep bool
	var dryRun bool
	var changedSince string

	c := &cobra.Command{
		Use:   "analyze <path>",
		Short: "Extract symbols from source files and upsert them as workspace elements",
		Long: `Walks the given path, extracts code symbols (functions, classes, types) using
tree-sitter grammar modules, and upserts each symbol as an Element in elements.yaml.
References found between symbols are upserted as Connectors in connectors.yaml.

By default only the given path is scanned.  Use --deep to scan the entire git repo
for cross-file call references.`,
		Args: cobra.ExactArgs(1),
		RunE: func(cmd *cobra.Command, args []string) error {
			scanPath := args[0]

			// Resolve to absolute path
			absPath, err := filepath.Abs(scanPath)
			if err != nil {
				return fmt.Errorf("resolve path: %w", err)
			}
			if _, err := os.Stat(absPath); err != nil {
				return fmt.Errorf("path %q not found: %w", scanPath, err)
			}

			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}

			repoScopes, err := resolveAnalyzeRepoScopes(ws, absPath)
			if err != nil {
				return err
			}

			ctx := cmd.Context()
			totalElements := 0
			totalConnectors := 0
			incrementalFiles := 0
			modeLabel := "shallow"
			if deep {
				modeLabel = "deep"
			}
			linePrefix := ""
			if dryRun {
				linePrefix = "[dry-run] "
			}
			fmt.Fprintf(cmd.OutOrStdout(), "%sAnalyzing %s (%s)...\n", linePrefix, scanPath, modeLabel)
			workspaceRoot, _ := filepath.Abs(ws.Dir)
			workspaceScan := samePath(absPath, workspaceRoot)

			for _, repoCtx := range repoScopes {
				rules := ws.IgnoreRulesForRepository(repoCtx.Name)
				scanRoot := absPath
				if workspaceScan {
					scanRoot = repoCtx.Root
				}

				// Detect git context (best-effort — not fatal if not a git repo)
				var repoURL, branch string
				if repoCtx.active() {
					if url, err := git.DetectRemoteURL(repoCtx.Root); err == nil {
						repoURL = url
					}
					if b, err := git.DetectBranch(repoCtx.Root); err == nil {
						branch = b
					}
				}

				// Extract symbols from the scan path
				scanResult, err := extractFromPath(ctx, scanRoot, rules)
				if err != nil {
					return fmt.Errorf("extract symbols: %w", err)
				}

				changedFileSet := map[string]struct{}{}
				if changedSince != "" && repoCtx.active() {
					changed, err := git.FilesChangedSince(repoCtx.Root, changedSince)
					if err != nil {
						return fmt.Errorf("git changed-since: %w", err)
					}
					incrementalFiles += len(changed)
					for _, file := range changed {
						changedFileSet[filepath.Clean(file)] = struct{}{}
					}
				}

				// Optionally scan entire repo for cross-file references
				if deep && repoCtx.active() && !workspaceScan {
					deepResult, err := extractFromPath(ctx, repoCtx.Root, rules)
					if err != nil {
						fmt.Fprintf(cmd.ErrOrStderr(), "Warning: deep scan failed: %v\n", err)
					} else {
						scanResult.Refs = append(scanResult.Refs, deepResult.Refs...)
					}
				}

				// Filter symbols by ignore rules
				filtered := filterSymbols(scanResult.Symbols, rules)
				if len(changedFileSet) > 0 {
					filtered = filterSymbolsByFiles(filtered, changedFileSet)
					scanResult.Refs = filterRefsByFiles(scanResult.Refs, changedFileSet)
				}

				if len(filtered) == 0 {
					continue
				}

				// Build a ref map: symbol name → element ref (for connector creation)
				refMap := make(map[string]string) // symbolName → ref slug

				repoElements := 0
				for _, sym := range filtered {
					ref := slugifySymbol(sym.Name, sym.FilePath, refMap)
					refMap[sym.Name] = ref

					relPath := sym.FilePath
					if repoCtx.active() {
						if rel, err := filepath.Rel(repoCtx.Root, sym.FilePath); err == nil {
							relPath = rel
						}
					}

					spec := &workspace.Element{
						Name:     sym.Name,
						Kind:     sym.Kind,
						Owner:    repoCtx.Name,
						FilePath: relPath,
						Symbol:   sym.Name,
						Repo:     repoURL,
						Branch:   branch,
					}

					if dryRun {
						repoElements++
						continue
					}

					if err := workspace.UpsertElement(*wdir, ref, spec); err != nil {
						fmt.Fprintf(cmd.ErrOrStderr(), "Warning: upsert element %q: %v\n", ref, err)
						continue
					}
					repoElements++
				}

				// Create connectors for discovered references
				repoConnectors := 0
				for _, ref := range scanResult.Refs {
					// Only create connectors between symbols we extracted
					if rules.ShouldIgnoreSymbol(ref.Name) {
						continue
					}
					toRef, toExists := refMap[ref.Name]
					if !toExists {
						continue // target symbol not in our extracted set
					}

					// Find which element's file this ref came from
					fromRef := refByFile(ref.FilePath, refMap, filtered)
					if fromRef == "" || fromRef == toRef {
						continue
					}

					connectorSpec := &workspace.Connector{
						View:         "root",
						Source:       fromRef,
						Target:       toRef,
						Label:        "calls",
						Relationship: "uses",
						Direction:    "forward",
					}

					if dryRun {
						repoConnectors++
						continue
					}

					if err := workspace.AppendConnector(*wdir, connectorSpec); err != nil {
						// Duplicate connectors produce a benign overwrite — not fatal
						_ = err
						continue
					}
					repoConnectors++
				}

				totalElements += repoElements
				totalConnectors += repoConnectors
			}

			fmt.Fprintf(cmd.OutOrStdout(), "%s  OK  %d elements written to elements.yaml\n", linePrefix, totalElements)
			fmt.Fprintf(cmd.OutOrStdout(), "%s  OK  %d connectors written to connectors.yaml\n", linePrefix, totalConnectors)
			fmt.Fprintf(cmd.OutOrStdout(), "%s  OK  %d repositories scanned\n", linePrefix, len(repoScopes))
			if changedSince != "" {
				fmt.Fprintf(cmd.OutOrStdout(), "%s  OK  Incremental scan: %d files changed since %s\n", linePrefix, incrementalFiles, changedSince)
			}
			fmt.Fprintln(cmd.OutOrStdout())
			if dryRun {
				fmt.Fprintf(cmd.OutOrStdout(), "%sNo files written. Remove --dry-run to apply.\n", linePrefix)
			} else {
				fmt.Fprintln(cmd.OutOrStdout(), "Tip: run `tld plan` to preview what will be applied.")
			}
			return nil
		},
	}

	c.Flags().BoolVar(&deep, "deep", false, "scan entire git repo for cross-file references (slower)")
	c.Flags().BoolVar(&dryRun, "dry-run", false, "print what would be written without modifying workspace")
	c.Flags().StringVar(&changedSince, "changed-since", "", "only re-analyse files changed since this git SHA")
	return c
}

// extractFromPath runs symbol extraction on a file or directory.
func extractFromPath(ctx context.Context, path string, rules *ignore.Rules) (*symbol.Result, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	if info.IsDir() {
		return symbol.ExtractDir(ctx, path, rules)
	}
	// Single file
	if rules.ShouldIgnoreFile(path) {
		return &symbol.Result{}, nil
	}
	return symbol.ExtractFile(ctx, path)
}

// slugifySymbol produces a unique ref slug for a symbol. If the base slug is already
// taken by a different symbol, it appends a file-based suffix.
func slugifySymbol(name, filePath string, existing map[string]string) string {
	base := workspace.Slugify(name)
	if _, taken := existing[base]; !taken {
		return base
	}
	// Append shortened file name to disambiguate
	fileBase := strings.TrimSuffix(filepath.Base(filePath), filepath.Ext(filePath))
	candidate := workspace.Slugify(fileBase + "-" + name)
	return candidate
}

// filterSymbols removes symbols that match the ignore rules.
func filterSymbols(symbols []symbol.Symbol, rules *ignore.Rules) []symbol.Symbol {
	var out []symbol.Symbol
	for _, s := range symbols {
		if rules.ShouldIgnoreSymbol(s.Name) {
			continue
		}
		out = append(out, s)
	}
	return out
}

func filterSymbolsByFiles(symbols []symbol.Symbol, changedFiles map[string]struct{}) []symbol.Symbol {
	var out []symbol.Symbol
	for _, sym := range symbols {
		if _, ok := changedFiles[filepath.Clean(sym.FilePath)]; ok {
			out = append(out, sym)
		}
	}
	return out
}

func filterRefsByFiles(refs []symbol.Ref, changedFiles map[string]struct{}) []symbol.Ref {
	var out []symbol.Ref
	for _, ref := range refs {
		if _, ok := changedFiles[filepath.Clean(ref.FilePath)]; ok {
			out = append(out, ref)
		}
	}
	return out
}

// refByFile returns the element ref for the first symbol extracted from the given file.
func refByFile(filePath string, refMap map[string]string, symbols []symbol.Symbol) string {
	for _, s := range symbols {
		if s.FilePath == filePath {
			if ref, ok := refMap[s.Name]; ok {
				return ref
			}
		}
	}
	return ""
}
