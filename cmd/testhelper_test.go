package cmd_test

import (
	"bytes"
	"io"
	"os"
	"strings"
	"testing"

	"github.com/mertcikla/tld-cli/cmd"
)

// runCmd executes a tld command rooted at dir with the given args.
// It returns stdout, stderr, and any error from Execute.
func runCmd(t *testing.T, dir string, args ...string) (stdout, stderr string, err error) {
	t.Helper()
	return runCmdWithStdin(t, dir, strings.NewReader(""), args...)
}

// mustRunCmd is like runCmd but fails the test on error.
func mustRunCmd(t *testing.T, dir string, args ...string) (stdout, stderr string) {
	t.Helper()
	stdout, stderr, err := runCmd(t, dir, args...)
	if err != nil {
		t.Fatalf("runCmd %v failed: %v\nstdout: %s\nstderr: %s", args, err, stdout, stderr)
	}
	return stdout, stderr
}

// runCmdWithStdin is like runCmd but allows injecting stdin content.
func runCmdWithStdin(t *testing.T, dir string, stdin io.Reader, args ...string) (stdout, stderr string, err error) {
	t.Helper()

	// Isolate configuration if not already set by the test
	if os.Getenv("TLD_CONFIG_DIR") == "" {
		configDir := t.TempDir()
		os.Setenv("TLD_CONFIG_DIR", configDir)
		t.Cleanup(func() { os.Unsetenv("TLD_CONFIG_DIR") })
	}

	root := cmd.NewRootCmd()
	outBuf, errBuf := &bytes.Buffer{}, &bytes.Buffer{}
	root.SetOut(outBuf)
	root.SetErr(errBuf)
	root.SetIn(stdin)
	root.SetArgs(append([]string{"--workspace", dir}, args...))
	err = root.Execute()
	return outBuf.String(), errBuf.String(), err
}

// mustInitWorkspace runs "tld init <dir>" and fails the test on error.
// init takes a positional arg, not --workspace, so we pass "." as the workspace.
func mustInitWorkspace(t *testing.T, dir string) {
	t.Helper()
	// "--workspace ." is ignored by init; the positional arg sets the target dir.
	_, _, err := runCmd(t, ".", "init", dir)
	if err != nil {
		t.Fatalf("init workspace: %v", err)
	}
}
