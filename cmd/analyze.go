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

			// Detect git context (best-effort — not fatal if not a git repo)
			var repoURL, branch string
			var repoRoot string
			if root, err := git.RepoRoot(absPath); err == nil {
				repoRoot = root
				if url, err := git.DetectRemoteURL(root); err == nil {
					repoURL = url
				}
				if b, err := git.DetectBranch(root); err == nil {
					branch = b
				}
			}

			ctx := cmd.Context()
			rules := ws.IgnoreRules

			// Extract symbols from the scan path
			scanResult, err := extractFromPath(ctx, absPath, rules)
			if err != nil {
				return fmt.Errorf("extract symbols: %w", err)
			}

			// Optionally scan entire repo for cross-file references
			if deep && repoRoot != "" {
				deepResult, err := extractFromPath(ctx, repoRoot, rules)
				if err != nil {
					fmt.Fprintf(cmd.ErrOrStderr(), "Warning: deep scan failed: %v\n", err)
				} else {
					scanResult.Refs = append(scanResult.Refs, deepResult.Refs...)
				}
			}

			// Filter symbols by ignore rules
			filtered := filterSymbols(scanResult.Symbols, rules)

			if len(filtered) == 0 {
				fmt.Fprintln(cmd.OutOrStdout(), "No symbols found.")
				return nil
			}

			// Build a ref map: symbol name → element ref (for connector creation)
			refMap := make(map[string]string) // symbolName → ref slug

			upserted := 0
			for _, sym := range filtered {
				ref := slugifySymbol(sym.Name, sym.FilePath, refMap)
				refMap[sym.Name] = ref

				relPath := sym.FilePath
				if repoRoot != "" {
					if rel, err := filepath.Rel(repoRoot, sym.FilePath); err == nil {
						relPath = rel
					}
				}

				spec := &workspace.Element{
					Name:     sym.Name,
					Kind:     sym.Kind,
					FilePath: relPath,
					Symbol:   sym.Name,
					Repo:     repoURL,
					Branch:   branch,
				}

				if dryRun {
					fmt.Fprintf(cmd.OutOrStdout(), "[dry-run] upsert element %q (kind=%s, file=%s)\n",
						ref, sym.Kind, relPath)
					upserted++
					continue
				}

				if err := workspace.UpsertElement(*wdir, ref, spec); err != nil {
					fmt.Fprintf(cmd.ErrOrStderr(), "Warning: upsert element %q: %v\n", ref, err)
					continue
				}
				upserted++
			}

			// Create connectors for discovered references
			connectors := 0
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
					fmt.Fprintf(cmd.OutOrStdout(), "[dry-run] upsert connector %s → %s\n", fromRef, toRef)
					connectors++
					continue
				}

				if err := workspace.AppendConnector(*wdir, connectorSpec); err != nil {
					// Duplicate connectors produce a benign overwrite — not fatal
					_ = err
					continue
				}
				connectors++
			}

			if dryRun {
				fmt.Fprintf(cmd.OutOrStdout(), "Dry run: %d elements, %d connectors (not written)\n",
					upserted, connectors)
			} else {
				fmt.Fprintf(cmd.OutOrStdout(), "Analyzed: upserted %d elements, %d connectors\n",
					upserted, connectors)
			}
			return nil
		},
	}

	c.Flags().BoolVar(&deep, "deep", false, "scan entire git repo for cross-file references (slower)")
	c.Flags().BoolVar(&dryRun, "dry-run", false, "print what would be written without modifying workspace")
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
