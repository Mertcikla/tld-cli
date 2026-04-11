package cmd

import (
	"fmt"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/mertcikla/tld-cli/internal/git"
	"github.com/mertcikla/tld-cli/workspace"
)

type repoScope struct {
	Root      string
	Name      string
	Label     string
	RemoteURL string
	Branch    string
}

func detectRepoScope(startDir, fallbackDir string) repoScope {
	for _, dir := range []string{startDir, fallbackDir} {
		if dir == "" {
			continue
		}
		root, err := git.RepoRoot(dir)
		if err != nil {
			continue
		}
		scope := repoScope{Root: root, Name: filepath.Base(root), Label: filepath.Base(root)}
		if url, err := git.DetectRemoteURL(root); err == nil {
			scope.RemoteURL = url
		}
		if branch, err := git.DetectBranch(root); err == nil {
			scope.Branch = branch
		}
		return scope
	}
	return repoScope{}
}

func (s repoScope) active() bool {
	return s.Root != ""
}

func (s repoScope) displayName() string {
	if s.Label != "" {
		return s.Label
	}
	if s.Root != "" {
		return filepath.Base(s.Root)
	}
	return "<unknown>"
}

func (s repoScope) resolvePath(filePath string) string {
	if filePath == "" {
		return ""
	}
	if filepath.IsAbs(filePath) || s.Root == "" {
		return filePath
	}
	return filepath.Join(s.Root, filePath)
}

func (s repoScope) matchesElement(element *workspace.Element) bool {
	if element == nil {
		return false
	}
	if !s.active() {
		return true
	}
	if s.RemoteURL != "" && element.Repo != "" {
		return element.Repo == s.RemoteURL
	}
	if element.FilePath == "" {
		return false
	}
	absolute := s.resolvePath(element.FilePath)
	if absolute == "" {
		return false
	}
	absolute = filepath.Clean(absolute)
	rel, err := filepath.Rel(s.Root, absolute)
	if err != nil {
		return false
	}
	return rel != ".." && !strings.HasPrefix(rel, ".."+string(os.PathSeparator))
}

func configuredRepoScopes(ws *workspace.Workspace) []repoScope {
	if ws == nil || ws.WorkspaceConfig == nil || len(ws.WorkspaceConfig.Repositories) == 0 {
		return nil
	}

	workspaceRoot, err := filepath.Abs(ws.Dir)
	if err != nil {
		return nil
	}

	seen := make(map[string]struct{})
	scopes := make([]repoScope, 0, len(ws.WorkspaceConfig.Repositories))
	for repoName, repository := range ws.WorkspaceConfig.Repositories {
		for _, candidate := range expandRepositoryCandidates(workspaceRoot, repository.LocalDir) {
			root, err := git.RepoRoot(candidate)
			if err != nil {
				continue
			}
			root = filepath.Clean(root)
			if _, ok := seen[root]; ok {
				continue
			}
			seen[root] = struct{}{}
			scopes = append(scopes, repoScope{
				Root:      root,
				Name:      repoName,
				Label:     repositoryLabel(workspaceRoot, root, repoName),
				RemoteURL: repository.URL,
			})
		}
	}

	sort.Slice(scopes, func(i, j int) bool {
		return scopes[i].Label < scopes[j].Label
	})
	return scopes
}

func resolveAnalyzeRepoScopes(ws *workspace.Workspace, absPath string) ([]repoScope, error) {
	hasConfiguredRepositories := ws != nil && ws.WorkspaceConfig != nil && len(ws.WorkspaceConfig.Repositories) > 0
	configured := configuredRepoScopes(ws)
	if hasConfiguredRepositories {
		workspaceRoot := ""
		if ws != nil {
			workspaceRoot, _ = filepath.Abs(ws.Dir)
		}
		if workspaceRoot != "" && samePath(absPath, workspaceRoot) {
			if len(configured) == 0 {
				return nil, fmt.Errorf("no configured repositories found under workspace repositories")
			}
			return configured, nil
		}
		for _, scope := range configured {
			if pathWithin(absPath, scope.Root) {
				return []repoScope{scope}, nil
			}
		}
		if len(configured) == 0 {
			return nil, fmt.Errorf("no configured repositories found under workspace repositories")
		}
		return nil, fmt.Errorf("path %q is not inside any configured repository", absPath)
	}

	scope := detectRepoScope(absPath, getWorkingDir())
	if !scope.active() {
		return nil, fmt.Errorf("no git repository found for %q", absPath)
	}
	return []repoScope{scope}, nil
}

func expandRepositoryCandidates(workspaceRoot, localDir string) []string {
	cleaned := filepath.Clean(localDir)
	if filepath.IsAbs(cleaned) {
		return []string{cleaned}
	}
	pattern := filepath.Join(workspaceRoot, localDir)
	if strings.ContainsAny(localDir, "*?[") {
		matches, err := filepath.Glob(pattern)
		if err != nil {
			return nil
		}
		return matches
	}
	return []string{pattern}
}

func repositoryLabel(workspaceRoot, root, repoName string) string {
	if rel, err := filepath.Rel(workspaceRoot, root); err == nil && rel != "." && !strings.HasPrefix(rel, "..") {
		return rel
	}
	if repoName != "" {
		return repoName
	}
	return filepath.Base(root)
}

func samePath(a, b string) bool {
	return filepath.Clean(a) == filepath.Clean(b)
}

func pathWithin(path, root string) bool {
	rel, err := filepath.Rel(root, path)
	if err != nil {
		return false
	}
	rel = filepath.Clean(rel)
	if rel == "." {
		return true
	}
	return rel != ".." && !strings.HasPrefix(rel, ".."+string(os.PathSeparator))
}

func (s repoScope) matchesWorkspaceRepo(ws *workspace.Workspace) bool {
	if ws == nil || ws.WorkspaceConfig == nil || len(ws.WorkspaceConfig.Repositories) == 0 {
		return true
	}
	if !s.active() {
		return false
	}

	workspaceRoot, err := filepath.Abs(ws.Dir)
	if err != nil {
		return false
	}
	repoRoot, err := filepath.Abs(s.Root)
	if err != nil {
		return false
	}
	rel, err := filepath.Rel(workspaceRoot, repoRoot)
	if err != nil {
		return false
	}
	rel = filepath.Clean(rel)
	if rel == "." {
		rel = ""
	}

	for _, repository := range ws.WorkspaceConfig.Repositories {
		trimmed := strings.TrimSuffix(repository.LocalDir, string(os.PathSeparator))
		if trimmed == "" {
			continue
		}
		if trimmed == rel || trimmed == filepath.Base(repoRoot) {
			return true
		}
		if matched, err := filepath.Match(trimmed, rel); err == nil && matched {
			return true
		}
		if strings.HasPrefix(rel, trimmed+string(os.PathSeparator)) {
			return true
		}
	}

	return false
}
