package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/workspace"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func protoPtr[T any](v T) *T {
	return &v
}

func TestExportCmd(t *testing.T) {
	svc := &mockDiagramService{
		exportFunc: func(_ *diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error) {
			resp := &diagv1.ExportOrganizationResponse{
				Diagrams: []*diagv1.Diagram{
					{Id: 1, Name: "D1", UpdatedAt: timestamppb.Now()},
				},
				Objects: []*diagv1.Object{
					{Id: 2, Name: "O1", Type: protoPtr("service"), UpdatedAt: timestamppb.Now()},
				},
				Placements: []*diagv1.ObjectPlacement{
					{DiagramId: 1, ElementId: 2, PositionX: 10, PositionY: 20},
				},
			}
			return resp, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	stdout, _, err := runCmd(t, dir, "export")
	if err != nil {
		t.Fatalf("export: %v", err)
	}

	if !strings.Contains(stdout, "Exported 1 elements, 0 diagrams, 0 connectors") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}

	// Verify files
	if _, err := os.Stat(filepath.Join(dir, "elements.yaml")); os.IsNotExist(err) {
		t.Error("elements.yaml not created")
	}
	if _, err := os.Stat(filepath.Join(dir, "connectors.yaml")); os.IsNotExist(err) {
		t.Error("connectors.yaml not created")
	}
}

func TestExportCmd_ElementOwnedDiagramBecomesView(t *testing.T) {
	now := timestamppb.Now()
	svc := &mockDiagramService{
		exportFunc: func(_ *diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error) {
			return &diagv1.ExportOrganizationResponse{
				Diagrams: []*diagv1.Diagram{
					{Id: 1, Name: "Workspace Root", UpdatedAt: now},
					{Id: 2, Name: "API Internals", LevelLabel: protoPtr("Container"), UpdatedAt: now},
				},
				Objects: []*diagv1.Object{
					{Id: 10, Name: "API", Type: protoPtr("service"), UpdatedAt: now},
					{Id: 11, Name: "Handler", Type: protoPtr("component"), UpdatedAt: now},
					{Id: 12, Name: "DB", Type: protoPtr("database"), UpdatedAt: now},
				},
				Placements: []*diagv1.ObjectPlacement{
					{DiagramId: 1, ElementId: 10, PositionX: 10, PositionY: 20},
					{DiagramId: 2, ElementId: 11, PositionX: 30, PositionY: 40},
					{DiagramId: 2, ElementId: 12, PositionX: 50, PositionY: 60},
				},
				Links: []*diagv1.ElementNavigation{
					{ElementId: 10, FromDiagramId: 1, ToDiagramId: 2},
				},
				Edges: []*diagv1.Edge{
					{Id: 30, DiagramId: 2, SourceElementId: 11, TargetElementId: 12, Label: protoPtr("queries"), UpdatedAt: now},
				},
			}, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	stdout, _, err := runCmd(t, dir, "export")
	if err != nil {
		t.Fatalf("export: %v", err)
	}
	if !strings.Contains(stdout, "Exported 3 elements, 1 diagrams, 1 connectors") {
		t.Fatalf("unexpected export output: %q", stdout)
	}

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("load workspace: %v", err)
	}

	api := ws.Elements["api"]
	if api == nil || !api.HasView {
		t.Fatalf("api element should own a diagram, got %#v", api)
	}
	if api.ViewLabel != "Container" {
		t.Fatalf("api view label = %q, want Container", api.ViewLabel)
	}
	if len(api.Placements) != 1 || api.Placements[0].ParentRef != "root" {
		t.Fatalf("api placements = %+v, want parent root", api.Placements)
	}

	handler := ws.Elements["handler"]
	if handler == nil || len(handler.Placements) != 1 || handler.Placements[0].ParentRef != "api" {
		t.Fatalf("handler placements = %+v, want parent api", handler)
	}

	connector := ws.Connectors["api:handler:db:queries"]
	if connector == nil {
		t.Fatalf("expected connector api:handler:db:queries, got %+v", ws.Connectors)
	}
	if connector.View != "api" {
		t.Fatalf("connector view = %q, want api", connector.View)
	}
}
