// Package workspace handles loading, validating, writing, and deleting workspace YAML files.
package workspace

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/mertcikla/tld-cli/internal/ignore"
	"gopkg.in/yaml.v3"
)

// Load reads the workspace from dir. The global configuration is read from tld.yaml.
func Load(dir string) (*Workspace, error) {
	ws := &Workspace{
		Dir:        dir,
		Elements:   make(map[string]*Element),
		Connectors: make(map[string]*Connector),
	}

	// Load config
	cfgPath, err := ConfigPath()
	if err != nil {
		return nil, fmt.Errorf("get config path: %w", err)
	}
	cfgData, err := os.ReadFile(cfgPath)
	if err != nil {
		return nil, fmt.Errorf("read tld.yaml: %w", err)
	}
	if err := yaml.Unmarshal(cfgData, &ws.Config); err != nil {
		return nil, fmt.Errorf("parse tld.yaml: %w", err)
	}
	// Fallback: TLD_API_KEY env var
	if ws.Config.APIKey == "" {
		ws.Config.APIKey = os.Getenv("TLD_API_KEY")
	}

	// Load workspace-local configuration from .tld.yaml if present.
	workspaceConfigPath := WorkspaceConfigPath(dir)
	if data, err := os.ReadFile(workspaceConfigPath); err == nil {
		cfg := &WorkspaceConfig{}
		if err := yaml.Unmarshal(data, cfg); err != nil {
			return nil, fmt.Errorf("parse .tld.yaml: %w", err)
		}
		for key, repo := range cfg.Repositories {
			if repo.Config == nil {
				repo.Config = &RepositoryConfig{}
			}
			if repo.Config.Mode == "" {
				repo.Config.Mode = "upsert"
			}
			cfg.Repositories[key] = repo
		}
		ws.WorkspaceConfig = cfg
		ws.IgnoreRules = &ignore.Rules{Exclude: append([]string{}, cfg.Exclude...)}
	} else if !os.IsNotExist(err) {
		return nil, fmt.Errorf("read .tld.yaml: %w", err)
	}

	// Load elements from elements.yaml
	elementsFile := filepath.Join(dir, "elements.yaml")
	if data, err := os.ReadFile(elementsFile); err == nil {
		if err := yaml.Unmarshal(data, &ws.Elements); err != nil {
			return nil, fmt.Errorf("parse elements.yaml: %w", err)
		}
		delete(ws.Elements, "_meta")
		delete(ws.Elements, "_meta_elements")
		delete(ws.Elements, "_meta_views")
	}

	// Load connectors from connectors.yaml
	connectorsFile := filepath.Join(dir, "connectors.yaml")
	if data, err := os.ReadFile(connectorsFile); err == nil {
		if err := yaml.Unmarshal(data, &ws.Connectors); err != nil {
			return nil, fmt.Errorf("parse connectors.yaml: %w", err)
		}
		delete(ws.Connectors, "_meta")
		delete(ws.Connectors, "_meta_connectors")
	}

	// Load metadata
	meta, err := LoadMetadata(dir)
	if err != nil {
		return nil, fmt.Errorf("load metadata: %w", err)
	}
	ws.Meta = meta

	return ws, nil
}
