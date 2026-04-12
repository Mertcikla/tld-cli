package cmd_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"testing"

	"github.com/mertcikla/tld-cli/cmd"
	"github.com/mertcikla/tld-cli/workspace"
)

func InitTestGitRepo(t *testing.T, dir string, filename string, source string) {
	t.Helper()
	if err := os.MkdirAll(dir, 0750); err != nil {
		t.Fatalf("MkdirAll: %v", err)
	}
	git := func(args ...string) {
		cmd := exec.Command("git", args...)
		cmd.Dir = dir
		cmd.Env = append(os.Environ(),
			"GIT_AUTHOR_NAME=Test",
			"GIT_AUTHOR_EMAIL=test@example.com",
			"GIT_COMMITTER_NAME=Test",
			"GIT_COMMITTER_EMAIL=test@example.com",
		)
		out, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}

	git("init", "-b", "main")
	if err := os.WriteFile(filepath.Join(dir, filename), []byte(source), 0600); err != nil {
		t.Fatalf("write %s: %v", filename, err)
	}
	git("add", filename)
	git("commit", "-m", "initial")
}

func TestRepoScopeMatchesWorkspaceRepositories(t *testing.T) {
	scope := cmd.RepoScope{Root: "/work/product/frontend"}
	ws := &workspace.Workspace{
		Dir: "/work/product",
		WorkspaceConfig: &workspace.WorkspaceConfig{
			Repositories: map[string]workspace.Repository{
				"frontend": {LocalDir: "frontend"},
				"payments": {LocalDir: "services/payments"},
			},
		},
	}

	if !scope.MatchesWorkspaceRepo(ws) {
		t.Fatal("expected frontend repo to match configured repositories")
	}

	other := cmd.RepoScope{Root: "/work/product/backend"}
	if other.MatchesWorkspaceRepo(ws) {
		t.Fatal("expected backend repo to be rejected by configured repositories")
	}
}

func TestRepoScopeMatchesWorkspaceRepositories_EmptyLocalDir(t *testing.T) {
	// workspace is at /work/product/.tld
	// parent is /work/product
	scope := cmd.RepoScope{Root: "/work/product"}
	ws := &workspace.Workspace{
		Dir: "/work/product/.tld",
		WorkspaceConfig: &workspace.WorkspaceConfig{
			Repositories: map[string]workspace.Repository{
				"root": {LocalDir: ""},
			},
		},
	}

	if !scope.MatchesWorkspaceRepo(ws) {
		t.Fatal("expected root repo (empty localDir) to match configured repositories")
	}
}

func TestExpandRepositoryCandidates_EmptyLocalDir(t *testing.T) {
	workspaceRoot := "/work/product/.tld"
	candidates := cmd.ExpandRepositoryCandidates(workspaceRoot, "")
	if len(candidates) != 1 {
		t.Fatalf("len(candidates) = %d, want 1", len(candidates))
	}
	if candidates[0] != "/work/product" {
		t.Fatalf("candidates[0] = %q, want %q", candidates[0], "/work/product")
	}
}

func TestResolveAnalyzeRepoScopes_FromWorkspaceRoot(t *testing.T) {
	workspaceDir := t.TempDir()
	InitTestGitRepo(t, filepath.Join(workspaceDir, "frontend"), "frontend.go", "package frontend\nfunc FrontendService() {}\n")
	InitTestGitRepo(t, filepath.Join(workspaceDir, "services", "payments"), "payments.go", "package payments\nfunc PaymentService() {}\n")

	ws := &workspace.Workspace{
		Dir: workspaceDir,
		WorkspaceConfig: &workspace.WorkspaceConfig{
			Repositories: map[string]workspace.Repository{
				"frontend": {LocalDir: "frontend"},
				"payments": {LocalDir: "services/payments"},
			},
		},
	}

	scopes := cmd.ConfiguredRepoScopes(ws)
	if len(scopes) != 2 {
		t.Fatalf("len(scopes) = %d, want 2", len(scopes))
	}

	resolved, err := cmd.ResolveAnalyzeRepoScopes(ws, workspaceDir)
	if err != nil {
		t.Fatalf("ResolveAnalyzeRepoScopes: %v", err)
	}
	if len(resolved) != 2 {
		t.Fatalf("len(resolved) = %d, want 2", len(resolved))
	}
}
