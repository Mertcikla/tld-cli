package workspace

import (
	"context"
	"fmt"
	"os"
	"strings"

	"github.com/mertcikla/tld-cli/internal/symbol"
)

// ValidationError describes a single validation failure.
type ValidationError struct {
	Location string
	Message  string
}

func (e ValidationError) Error() string {
	return fmt.Sprintf("%s: %s", e.Location, e.Message)
}

// Validate checks cross-reference integrity, required fields, and cycle
// detection in parent_diagram chains.
func (ws *Workspace) Validate() []ValidationError {
	var errs []ValidationError
	errs = append(errs, ws.validateConflictMarkers()...)

	// Elements: required fields + placement refs
	elementNames := make(map[string]string)
	for ref, element := range ws.Elements {
		loc := fmt.Sprintf("elements.yaml[%s]", ref)
		if element.Name == "" {
			errs = append(errs, ValidationError{loc, "name is required"})
		} else {
			if existingRef, ok := elementNames[element.Name]; ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("duplicate element name %q (also used by %q)", element.Name, existingRef)})
			}
			elementNames[element.Name] = ref
		}
		if element.Kind == "" {
			errs = append(errs, ValidationError{loc, "kind is required"})
		}
		if element.Owner != "" && ws.WorkspaceConfig != nil {
			if _, ok := ws.WorkspaceConfig.Repositories[element.Owner]; !ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("owner %q is not a registered repository", element.Owner)})
			}
		}
		for index, placement := range element.Placements {
			ploc := fmt.Sprintf("elements.yaml[%s][placements][%d]", ref, index)
			if placement.ParentRef == "" {
				errs = append(errs, ValidationError{ploc, "parent is required"})
				continue
			}
			if placement.ParentRef != "root" {
				if _, ok := ws.Elements[placement.ParentRef]; !ok {
					errs = append(errs, ValidationError{ploc, fmt.Sprintf("parent ref %q not found", placement.ParentRef)})
				}
			}
		}
	}

	// Connectors: required fields + ref integrity
	for ref, connector := range ws.Connectors {
		loc := fmt.Sprintf("connectors.yaml[%s]", ref)
		if connector.View == "" {
			errs = append(errs, ValidationError{loc, "view is required"})
		} else if connector.View != "root" {
			if _, ok := ws.Elements[connector.View]; !ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("view ref %q not found", connector.View)})
			}
		}
		if connector.Source == "" {
			errs = append(errs, ValidationError{loc, "source is required"})
		} else if _, ok := ws.Elements[connector.Source]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("source ref %q not found", connector.Source)})
		}
		if connector.Target == "" {
			errs = append(errs, ValidationError{loc, "target is required"})
		} else if _, ok := ws.Elements[connector.Target]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("target ref %q not found", connector.Target)})
		}
	}

	// Diagrams: required fields + parent ref integrity + unique names
	diagramNames := make(map[string]string) // name -> ref
	for ref, d := range ws.Diagrams {
		loc := fmt.Sprintf("diagrams.yaml[%s]", ref)
		if d.Name == "" {
			errs = append(errs, ValidationError{loc, "name is required"})
		} else {
			if existingRef, ok := diagramNames[d.Name]; ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("duplicate diagram name %q (also used by %q)", d.Name, existingRef)})
			}
			diagramNames[d.Name] = ref
		}
		if d.ParentDiagram != "" {
			if _, ok := ws.Diagrams[d.ParentDiagram]; !ok {
				errs = append(errs, ValidationError{
					loc,
					fmt.Sprintf("parent_diagram %q not found", d.ParentDiagram),
				})
			}
		}
	}
	// Cycle detection in parent chains
	for ref := range ws.Diagrams {
		if cycle := detectParentCycle(ws, ref); cycle != "" {
			errs = append(errs, ValidationError{
				fmt.Sprintf("diagrams.yaml[%s]", ref),
				fmt.Sprintf("circular parent_diagram chain: %s", cycle),
			})
		}
	}

	// Objects: required fields + diagram placement refs + unique names
	objectNames := make(map[string]string) // name -> ref
	for ref, o := range ws.Objects {
		loc := fmt.Sprintf("objects.yaml[%s]", ref)
		if o.Name == "" {
			errs = append(errs, ValidationError{loc, "name is required"})
		} else {
			if existingRef, ok := objectNames[o.Name]; ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("duplicate object name %q (also used by %q)", o.Name, existingRef)})
			}
			objectNames[o.Name] = ref
		}
		if o.Type == "" {
			errs = append(errs, ValidationError{loc, "type is required"})
		}
		for i, p := range o.Diagrams {
			ploc := fmt.Sprintf("objects.yaml[%s][diagrams][%d]", ref, i)
			if p.Diagram == "" {
				errs = append(errs, ValidationError{ploc, "diagram is required"})
				continue
			}
			if _, ok := ws.Diagrams[p.Diagram]; !ok {
				errs = append(errs, ValidationError{
					ploc,
					fmt.Sprintf("diagram ref %q not found", p.Diagram),
				})
			}
		}
	}

	// Edges: required fields + ref integrity
	for ref, e := range ws.Edges {
		loc := fmt.Sprintf("edges.yaml[%s]", ref)
		if e.Diagram == "" {
			errs = append(errs, ValidationError{loc, "diagram is required"})
		} else if _, ok := ws.Diagrams[e.Diagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("diagram ref %q not found", e.Diagram)})
		}
		if e.SourceObject == "" {
			errs = append(errs, ValidationError{loc, "source_object is required"})
		} else if _, ok := ws.Objects[e.SourceObject]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("source_object ref %q not found", e.SourceObject)})
		}
		if e.TargetObject == "" {
			errs = append(errs, ValidationError{loc, "target_object is required"})
		} else if _, ok := ws.Objects[e.TargetObject]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("target_object ref %q not found", e.TargetObject)})
		}
	}

	// Links: required fields + ref integrity
	for i, l := range ws.Links {
		loc := fmt.Sprintf("links.yaml[%d]", i)
		if l.Object != "" {
			if _, ok := ws.Objects[l.Object]; !ok {
				errs = append(errs, ValidationError{loc, fmt.Sprintf("object ref %q not found", l.Object)})
			}
		}
		if l.FromDiagram == "" {
			errs = append(errs, ValidationError{loc, "from_diagram is required"})
		} else if _, ok := ws.Diagrams[l.FromDiagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("from_diagram ref %q not found", l.FromDiagram)})
		}
		if l.ToDiagram == "" {
			errs = append(errs, ValidationError{loc, "to_diagram is required"})
		} else if _, ok := ws.Diagrams[l.ToDiagram]; !ok {
			errs = append(errs, ValidationError{loc, fmt.Sprintf("to_diagram ref %q not found", l.ToDiagram)})
		}
	}

	// Symbol verification: for elements that declare both file_path and symbol,
	// confirm the named symbol actually exists in the file (skip if file not locally accessible).
	errs = append(errs, ws.validateSymbols()...)

	return errs
}

func (ws *Workspace) validateConflictMarkers() []ValidationError {
	var errs []ValidationError
	checkString := func(location, value string) {
		if strings.Contains(value, "<<< LOCAL") || strings.Contains(value, ">>> SERVER") {
			errs = append(errs, ValidationError{Location: location, Message: "unresolved merge conflict"})
		}
	}

	for ref, diagram := range ws.Diagrams {
		loc := fmt.Sprintf("diagrams.yaml[%s]", ref)
		checkString(loc, diagram.Name)
		checkString(loc, diagram.Description)
	}
	for ref, object := range ws.Objects {
		loc := fmt.Sprintf("objects.yaml[%s]", ref)
		checkString(loc, object.Name)
		checkString(loc, object.Description)
		checkString(loc, object.Technology)
		checkString(loc, object.URL)
	}
	for ref, element := range ws.Elements {
		loc := fmt.Sprintf("elements.yaml[%s]", ref)
		checkString(loc, element.Name)
		checkString(loc, element.Description)
		checkString(loc, element.Technology)
		checkString(loc, element.URL)
	}
	for ref, connector := range ws.Connectors {
		loc := fmt.Sprintf("connectors.yaml[%s]", ref)
		checkString(loc, connector.Label)
		checkString(loc, connector.Description)
		checkString(loc, connector.Relationship)
	}
	return errs
}

// validateSymbols checks that elements with both file_path and symbol fields
// have a symbol that is actually present in the referenced file.
// Files that do not exist locally are silently skipped.
func (ws *Workspace) validateSymbols() []ValidationError {
	var errs []ValidationError
	ctx := context.Background()

	for ref, element := range ws.Elements {
		if element.FilePath == "" || element.Symbol == "" {
			continue
		}
		if _, err := os.Stat(element.FilePath); err != nil {
			continue // file not accessible locally — skip
		}
		found, err := symbol.HasSymbol(ctx, element.FilePath, element.Symbol)
		if err != nil {
			if _, unsupported := err.(symbol.ErrUnsupportedLanguage); unsupported {
				continue // language not supported — skip silently
			}
			errs = append(errs, ValidationError{
				Location: fmt.Sprintf("elements.yaml[%s]", ref),
				Message:  fmt.Sprintf("symbol verification failed: %v", err),
			})
			continue
		}
		if !found {
			errs = append(errs, ValidationError{
				Location: fmt.Sprintf("elements.yaml[%s]", ref),
				Message:  fmt.Sprintf("symbol %q not found in %s", element.Symbol, element.FilePath),
			})
		}
	}
	return errs
}

// detectParentCycle returns a string describing the cycle if one is found
// starting from startRef, or "" if no cycle.
func detectParentCycle(ws *Workspace, startRef string) string {
	visited := map[string]bool{}
	cur := startRef
	path := []string{cur}
	for {
		d, ok := ws.Diagrams[cur]
		if !ok || d.ParentDiagram == "" {
			return ""
		}
		if visited[d.ParentDiagram] {
			return fmt.Sprintf("%v -> %s", path, d.ParentDiagram)
		}
		visited[cur] = true
		cur = d.ParentDiagram
		path = append(path, cur)
	}
}
