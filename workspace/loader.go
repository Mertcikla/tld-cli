// Package workspace handles loading, validating, writing, and deleting workspace YAML files.
package workspace

import (
	"fmt"
	"os"
	"path/filepath"

	"gopkg.in/yaml.v3"
)

// Load reads the workspace from dir. The global configuration is read from tld.yaml.
func Load(dir string) (*Workspace, error) {
	ws := &Workspace{
		Dir:        dir,
		Diagrams:   make(map[string]*Diagram),
		Objects:    make(map[string]*Object),
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

	// Load diagrams from diagrams.yaml
	diagPath := filepath.Join(dir, "diagrams.yaml")
	if data, err := os.ReadFile(diagPath); err == nil {
		if err := yaml.Unmarshal(data, &ws.Diagrams); err != nil {
			return nil, fmt.Errorf("parse diagrams.yaml: %w", err)
		}
		delete(ws.Diagrams, "_meta")
	}

	// Load objects from objects.yaml
	objPath := filepath.Join(dir, "objects.yaml")
	if data, err := os.ReadFile(objPath); err == nil {
		if err := yaml.Unmarshal(data, &ws.Objects); err != nil {
			return nil, fmt.Errorf("parse objects.yaml: %w", err)
		}
		delete(ws.Objects, "_meta")
	}

	// Load edges from edges.yaml
	edgesFile := filepath.Join(dir, "edges.yaml")
	if data, err := os.ReadFile(edgesFile); err == nil {
		ws.Edges = make(map[string]*Edge)
		if err := yaml.Unmarshal(data, &ws.Edges); err != nil {
			return nil, fmt.Errorf("parse edges.yaml: %w", err)
		}
		delete(ws.Edges, "_meta")
	}

	// Load links from links.yaml
	linksFile := filepath.Join(dir, "links.yaml")
	if data, err := os.ReadFile(linksFile); err == nil {
		if err := yaml.Unmarshal(data, &ws.Links); err != nil {
			return nil, fmt.Errorf("parse links.yaml: %w", err)
		}
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
