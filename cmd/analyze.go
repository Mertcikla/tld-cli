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

				var repoURL, branch string
				if repoCtx.active() {
					if url, err := git.DetectRemoteURL(repoCtx.Root); err == nil {
						repoURL = url
					}
					if b, err := git.DetectBranch(repoCtx.Root); err == nil {
						branch = b
					}
				}

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

				if deep && repoCtx.active() && !workspaceScan {
					deepResult, err := extractFromPath(ctx, repoCtx.Root, rules)
					if err != nil {
						fmt.Fprintf(cmd.ErrOrStderr(), "Warning: deep scan failed: %v\n", err)
					} else {
						scanResult.Refs = append(scanResult.Refs, deepResult.Refs...)
					}
				}

				filtered := filterSymbols(scanResult.Symbols, rules)
				if len(changedFileSet) > 0 {
					filtered = filterSymbolsByFiles(filtered, changedFileSet)
					scanResult.Refs = filterRefsByFiles(scanResult.Refs, changedFileSet)
				}

				if len(filtered) == 0 {
					continue
				}

				usedRefs := make(map[string]struct{}, len(ws.Elements))
				for ref := range ws.Elements {
					usedRefs[ref] = struct{}{}
				}

				repoName := filepath.Base(repoCtx.Root)
				repoRef, err := ensureAnalyzeElement(*wdir, dryRun, ws, usedRefs, analyzeElementSpec{
					Name:      repoName,
					Kind:      "repository",
					Owner:     repoCtx.Name,
					Repo:      repoURL,
					Branch:    branch,
					HasView:   true,
					ViewLabel: repoName,
					ParentRef: "root",
					Identity: analyzeElementIdentity{
						Repo:     repoURL,
						Branch:   branch,
						FilePath: "",
						Symbol:   "",
						Kind:     "repository",
						Name:     repoName,
					},
				})
				if err != nil {
					return fmt.Errorf("ensure repository element: %w", err)
				}

				fileRefs := make(map[string]string)
				symbolRefs := make(map[string]string)
				symbolFiles := make(map[string]string)
				repoElements := 1

				for _, relPath := range uniqueFilePaths(filtered, repoCtx.Root, repoCtx.active()) {
					fileName := filepath.Base(relPath)
					fileRef, err := ensureAnalyzeElement(*wdir, dryRun, ws, usedRefs, analyzeElementSpec{
						Name:      fileName,
						Kind:      "file",
						Owner:     repoCtx.Name,
						Repo:      repoURL,
						Branch:    branch,
						FilePath:  relPath,
						HasView:   true,
						ViewLabel: fileName,
						ParentRef: repoRef,
						Identity: analyzeElementIdentity{
							Repo:     repoURL,
							Branch:   branch,
							FilePath: relPath,
							Symbol:   "",
							Kind:     "file",
							Name:     fileName,
						},
					})
					if err != nil {
						return fmt.Errorf("ensure file element %q: %w", relPath, err)
					}
					fileRefs[relPath] = fileRef
					repoElements++
				}

				for _, sym := range filtered {
					relPath := sym.FilePath
					if repoCtx.active() {
						if rel, err := filepath.Rel(repoCtx.Root, sym.FilePath); err == nil {
							relPath = rel
						}
					}
					relPath = filepath.Clean(relPath)
					fileRef := fileRefs[relPath]
					if fileRef == "" {
						continue
					}

					ref, err := ensureAnalyzeElement(*wdir, dryRun, ws, usedRefs, analyzeElementSpec{
						Name:      sym.Name,
						Kind:      sym.Kind,
						Owner:     repoCtx.Name,
						Repo:      repoURL,
						Branch:    branch,
						FilePath:  relPath,
						Symbol:    sym.Name,
						ParentRef: fileRef,
						Identity: analyzeElementIdentity{
							Repo:     repoURL,
							Branch:   branch,
							FilePath: relPath,
							Symbol:   sym.Name,
							Kind:     sym.Kind,
							Name:     sym.Name,
						},
					})
					if err != nil {
						fmt.Fprintf(cmd.ErrOrStderr(), "Warning: upsert element %q: %v\n", sym.Name, err)
						continue
					}
					symbolRefs[sym.Name] = ref
					symbolFiles[ref] = relPath
					repoElements++
				}

				repoConnectors := 0
				for _, ref := range scanResult.Refs {
					if rules.ShouldIgnoreSymbol(ref.Name) {
						continue
					}
					toRef, ok := symbolRefs[ref.Name]
					if !ok {
						continue
					}

					fromRef := refByFile(ref.FilePath, symbolRefs, filtered)
					if fromRef == "" || fromRef == toRef {
						continue
					}

					viewRef := repoRef
					if sourceFile := symbolFiles[fromRef]; sourceFile != "" && sourceFile == symbolFiles[toRef] {
						if fileRef := fileRefs[sourceFile]; fileRef != "" {
							viewRef = fileRef
						}
					}

					connectorSpec := &workspace.Connector{
						View:         viewRef,
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

func extractFromPath(ctx context.Context, path string, rules *ignore.Rules) (*symbol.Result, error) {
	info, err := os.Stat(path)
	if err != nil {
		return nil, err
	}
	if info.IsDir() {
		return symbol.ExtractDir(ctx, path, rules)
	}
	if rules.ShouldIgnoreFile(path) {
		return &symbol.Result{}, nil
	}
	return symbol.ExtractFile(ctx, path)
}

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

func refByFile(filePath string, refMap map[string]string, symbols []symbol.Symbol) string {
	for _, s := range symbols {
		if filepath.Clean(s.FilePath) == filepath.Clean(filePath) {
			if ref, ok := refMap[s.Name]; ok {
				return ref
			}
		}
	}
	return ""
}

type analyzeElementIdentity struct {
	Repo     string
	Branch   string
	FilePath string
	Symbol   string
	Kind     string
	Name     string
}

type analyzeElementSpec struct {
	Name      string
	Kind      string
	Owner     string
	Repo      string
	Branch    string
	FilePath  string
	Symbol    string
	HasView   bool
	ViewLabel string
	ParentRef string
	Identity  analyzeElementIdentity
}

func ensureAnalyzeElement(wdir string, dryRun bool, ws *workspace.Workspace, usedRefs map[string]struct{}, spec analyzeElementSpec) (string, error) {
	if ref, ok := findAnalyzeElementRef(ws, spec.Identity); ok {
		if dryRun {
			return ref, nil
		}
		return ref, workspace.UpsertElement(wdir, ref, analyzeElementToWorkspaceElement(spec))
	}

	ref := uniqueAnalyzeRef(spec.Name, spec.FilePath, usedRefs)
	usedRefs[ref] = struct{}{}
	if dryRun {
		return ref, nil
	}
	if err := workspace.UpsertElement(wdir, ref, analyzeElementToWorkspaceElement(spec)); err != nil {
		return "", err
	}
	return ref, nil
}

func analyzeElementToWorkspaceElement(spec analyzeElementSpec) *workspace.Element {
	return &workspace.Element{
		Name:      spec.Name,
		Kind:      spec.Kind,
		Owner:     spec.Owner,
		Repo:      spec.Repo,
		Branch:    spec.Branch,
		FilePath:  spec.FilePath,
		Symbol:    spec.Symbol,
		HasView:   spec.HasView,
		ViewLabel: spec.ViewLabel,
		Placements: []workspace.ViewPlacement{{
			ParentRef: spec.ParentRef,
		}},
	}
}

func findAnalyzeElementRef(ws *workspace.Workspace, identity analyzeElementIdentity) (string, bool) {
	for ref, element := range ws.Elements {
		if element == nil {
			continue
		}
		if identity.Repo != "" && element.Repo != identity.Repo {
			continue
		}
		if identity.Branch != "" && element.Branch != identity.Branch {
			continue
		}
		if filepath.Clean(element.FilePath) != filepath.Clean(identity.FilePath) {
			continue
		}
		if element.Symbol != identity.Symbol {
			continue
		}
		if identity.Kind != "" && element.Kind != identity.Kind {
			continue
		}
		if identity.Name != "" && element.Name != identity.Name {
			continue
		}
		return ref, true
	}
	return "", false
}

func uniqueAnalyzeRef(name, filePath string, used map[string]struct{}) string {
	base := workspace.Slugify(name)
	if base == "" {
		base = "element"
	}
	if _, taken := used[base]; !taken {
		return base
	}
	fileBase := strings.TrimSuffix(filepath.Base(filePath), filepath.Ext(filePath))
	candidate := workspace.Slugify(fileBase + "-" + name)
	if candidate == "" {
		candidate = base
	}
	if _, taken := used[candidate]; !taken {
		return candidate
	}
	for i := 2; ; i++ {
		withSuffix := fmt.Sprintf("%s-%d", candidate, i)
		if _, taken := used[withSuffix]; !taken {
			return withSuffix
		}
	}
}

func uniqueFilePaths(symbols []symbol.Symbol, repoRoot string, activeRepo bool) []string {
	seen := make(map[string]struct{})
	paths := make([]string, 0, len(symbols))
	for _, sym := range symbols {
		relPath := sym.FilePath
		if activeRepo {
			if rel, err := filepath.Rel(repoRoot, sym.FilePath); err == nil {
				relPath = rel
			}
		}
		relPath = filepath.Clean(relPath)
		if _, ok := seen[relPath]; ok {
			continue
		}
		seen[relPath] = struct{}{}
		paths = append(paths, relPath)
	}
	return paths
}
