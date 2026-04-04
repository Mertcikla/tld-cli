package workspace

import (
	"fmt"
	"time"

	hashidlib "github.com/mertcikla/tld-cli/internal/hashids"
)

// Config is parsed from .tld.yaml.
type Config struct {
	ServerURL  string            `yaml:"server_url"`
	APIKey     string            `yaml:"api_key"`
	OrgID      string            `yaml:"org_id"`
	Validation *ValidationConfig `yaml:"validation,omitempty"`
}

// ValidationConfig represents diagram validation settings.
type ValidationConfig struct {
	Level           int  `yaml:"level"`
	AllowLowInsight bool `yaml:"allow_low_insight"`
}

// Diagram represents an entry in diagrams.yaml
type Diagram struct {
	Name          string `yaml:"name"`
	Description   string `yaml:"description,omitempty"`
	LevelLabel    string `yaml:"level_label,omitempty"`
	ParentDiagram string `yaml:"parent_diagram,omitempty"`
}

// Placement is a diagram placement within an Object
type Placement struct {
	Diagram   string  `yaml:"diagram"`
	PositionX float64 `yaml:"position_x,omitempty"`
	PositionY float64 `yaml:"position_y,omitempty"`
}

// Object represents an entry in objects.yaml
type Object struct {
	Name        string      `yaml:"name"`
	Type        string      `yaml:"type"`
	Description string      `yaml:"description,omitempty"`
	Technology  string      `yaml:"technology,omitempty"`
	URL         string      `yaml:"url,omitempty"`
	LogoURL     string      `yaml:"logo_url,omitempty"`
	Repo        string      `yaml:"repo,omitempty"`
	Branch      string      `yaml:"branch,omitempty"`
	Language    string      `yaml:"language,omitempty"`
	FilePath    string      `yaml:"file_path,omitempty"`
	Diagrams    []Placement `yaml:"diagrams,omitempty"`
}

// Edge is one entry in edges.yaml
type Edge struct {
	Diagram          string `yaml:"diagram"`
	SourceObject     string `yaml:"source_object"`
	TargetObject     string `yaml:"target_object"`
	Label            string `yaml:"label,omitempty"`
	Description      string `yaml:"description,omitempty"`
	RelationshipType string `yaml:"relationship_type,omitempty"`
	Direction        string `yaml:"direction,omitempty"`
	EdgeType         string `yaml:"edge_type,omitempty"`
	URL              string `yaml:"url,omitempty"`
	SourceHandle     string `yaml:"source_handle,omitempty"`
	TargetHandle     string `yaml:"target_handle,omitempty"`
}

// Link is one entry in links.yaml
type Link struct {
	Object      string `yaml:"object,omitempty"`
	FromDiagram string `yaml:"from_diagram"`
	ToDiagram   string `yaml:"to_diagram"`
}

// ResourceID is an int32 that serializes to Hashids in YAML
type ResourceID int32

// MarshalYAML serialises a ResourceID to its Hashid string representation.
func (r ResourceID) MarshalYAML() (any, error) {
	if r == 0 {
		return nil, nil
	}
	return hashidlib.Encode(int32(r)), nil
}

// UnmarshalYAML deserialises a Hashid string back to a ResourceID.
func (r *ResourceID) UnmarshalYAML(unmarshal func(any) error) error {
	var s string
	if err := unmarshal(&s); err != nil {
		return err
	}
	if s == "" {
		*r = 0
		return nil
	}
	id, err := hashidlib.Decode(s)
	if err != nil {
		return fmt.Errorf("decode resource id: %w", err)
	}
	*r = ResourceID(id)
	return nil
}

// ResourceMetadata tracks system IDs and timestamps for resources
type ResourceMetadata struct {
	ID        ResourceID `yaml:"id"`
	UpdatedAt time.Time  `yaml:"updated_at"`
	Conflict  bool       `yaml:"conflict,omitempty"` // True if both local and server changed since last sync
}

// LockFile tracks workspace versioning and change history
type LockFile struct {
	Version       string          `yaml:"version"`    // "v1"
	VersionID     string          `yaml:"version_id"` // Workspace version UUID
	LastApply     time.Time       `yaml:"last_apply"`
	AppliedBy     string          `yaml:"applied_by"` // "cli" or "frontend"
	Resources     *ResourceCounts `yaml:"resources"`
	WorkspaceHash string          `yaml:"workspace_hash"`           // Hash of all YAML files
	ParentVersion *string         `yaml:"parent_version,omitempty"` // Previous version
	Metadata      *Meta           `yaml:"metadata,omitempty"`       // Metadata at time of last sync
}

// ResourceCounts holds diagram, object, edge, and link counts for a workspace version.
type ResourceCounts struct {
	Diagrams int `yaml:"diagrams"`
	Objects  int `yaml:"objects"`
	Edges    int `yaml:"edges"`
	Links    int `yaml:"links"`
}

// Workspace holds the fully loaded workspace state
type Workspace struct {
	Dir      string
	Config   Config
	Diagrams map[string]*Diagram // key = ref
	Objects  map[string]*Object  // key = ref
	Edges    map[string]*Edge
	Links    []Link
	Meta     *Meta // Loaded from separate _meta sections
}

// Meta contains metadata for all resources in the workspace.
type Meta struct {
	Diagrams map[string]*ResourceMetadata
	Objects  map[string]*ResourceMetadata
	Edges    map[string]*ResourceMetadata // key = "diagramRef:srcRef:tgtRef:label"
}
