package cmd_test

import (
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"testing"
)

func initGitRepo(t *testing.T, dir string, filename string, source string) {
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

func TestAnalyzeCmd_RejectsRepoOutsideConfiguredRepositories(t *testing.T) {
	workspaceDir := t.TempDir()
	mustInitWorkspace(t, workspaceDir)

	workspaceCfg := strings.Join([]string{
		"project_name: Demo",
		"repositories:",
		"  frontend:",
		"    url: github.com/example/frontend",
		"    localDir: frontend",
		"exclude: []",
		"",
	}, "\n")
	if err := os.WriteFile(filepath.Join(workspaceDir, ".tld.yaml"), []byte(workspaceCfg), 0600); err != nil {
		t.Fatalf("write workspace config: %v", err)
	}
	initGitRepo(t, filepath.Join(workspaceDir, "frontend"), "frontend.go", "package frontend\nfunc FrontendService() {}\n")

	repoDir := t.TempDir()
	git := func(args ...string) {
		cmd := exec.Command("git", args...)
		cmd.Dir = repoDir
		cmd.Env = append(os.Environ(), "GIT_AUTHOR_NAME=Test", "GIT_AUTHOR_EMAIL=test@example.com", "GIT_COMMITTER_NAME=Test", "GIT_COMMITTER_EMAIL=test@example.com")
		out, err := cmd.CombinedOutput()
		if err != nil {
			t.Fatalf("git %v: %v\n%s", args, err, out)
		}
	}

	git("init", "-b", "main")
	if err := os.WriteFile(filepath.Join(repoDir, "main.go"), []byte("package main\n"), 0600); err != nil {
		t.Fatalf("write main.go: %v", err)
	}
	git("add", "main.go")
	git("commit", "-m", "initial")

	stdout, stderr, err := runCmd(t, workspaceDir, "analyze", filepath.Join(repoDir, "main.go"))
	if err == nil {
		t.Fatalf("expected analyze to fail for repo outside configured repositories\nstdout: %s\nstderr: %s", stdout, stderr)
	}
	if !strings.Contains(err.Error(), "repo") || !strings.Contains(err.Error(), "repository") {
		t.Fatalf("unexpected error: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if strings.Contains(stdout, "Analyzed:") {
		t.Fatalf("analyze should not write workspace changes\nstdout: %s\nstderr: %s", stdout, stderr)
	}
}

func TestAnalyzeCmd_DiscoversConfiguredRepositories(t *testing.T) {
	workspaceDir := t.TempDir()
	mustInitWorkspace(t, workspaceDir)

	workspaceCfg := strings.Join([]string{
		"project_name: Demo",
		"repositories:",
		"  frontend:",
		"    url: github.com/example/frontend",
		"    localDir: frontend",
		"  backend:",
		"    url: github.com/example/backend",
		"    localDir: backend",
		"exclude: []",
		"",
	}, "\n")
	if err := os.WriteFile(filepath.Join(workspaceDir, ".tld.yaml"), []byte(workspaceCfg), 0600); err != nil {
		t.Fatalf("write workspace config: %v", err)
	}

	initGitRepo(t, filepath.Join(workspaceDir, "frontend"), "frontend.go", "package frontend\nfunc FrontendService() {}\n")
	initGitRepo(t, filepath.Join(workspaceDir, "backend"), "backend.go", "package backend\nfunc BackendService() {}\n")

	stdout, stderr, err := runCmd(t, workspaceDir, "analyze", workspaceDir, "--dry-run")
	if err != nil {
		t.Fatalf("analyze: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "Scanning repo frontend") || !strings.Contains(stdout, "Scanning repo backend") {
		t.Fatalf("stdout does not show both repo scopes\nstdout: %s\nstderr: %s", stdout, stderr)
	}
	if !strings.Contains(stdout, "[dry-run] [frontend]") || !strings.Contains(stdout, "[dry-run] [backend]") {
		t.Fatalf("stdout does not show repo-labeled dry-run output\nstdout: %s\nstderr: %s", stdout, stderr)
	}
	if !strings.Contains(stdout, "Dry run: 2 repos") {
		t.Fatalf("stdout does not summarize both repos\nstdout: %s\nstderr: %s", stdout, stderr)
	}
}
