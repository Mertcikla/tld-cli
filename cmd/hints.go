package cmd

import (
	"errors"
	"fmt"
	"strings"

	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/workspace"
)

func withHint(err error, hint string) error {
	return fmt.Errorf("%w\n  Hint: %s", err, hint)
}

func loadWorkspaceWithHint(dir string) (*workspace.Workspace, error) {
	ws, err := workspace.Load(dir)
	if err != nil {
		return nil, withHint(fmt.Errorf("load workspace: %w", err), "Run 'tld init' to create a workspace in this directory.")
	}
	return ws, nil
}

func ensureAPIKey(apiKey string) error {
	if strings.TrimSpace(apiKey) == "" {
		return withHint(errors.New("api_key is empty"), "Run 'tld login' or set the TLD_API_KEY environment variable.")
	}
	return nil
}

func orgIDRequiredHint(message string) error {
	return withHint(errors.New(message), "Run 'tld login' to set your org-id automatically.")
}

func withUnauthorizedHint(prefix string, err error) error {
	var connectErr *connect.Error
	if errors.As(err, &connectErr) && connectErr.Code() == connect.CodeUnauthenticated {
		return withHint(fmt.Errorf("%s: %w", prefix, err), "Your API key may have expired. Run 'tld login' to refresh it.")
	}
	return fmt.Errorf("%s: %w", prefix, err)
}
