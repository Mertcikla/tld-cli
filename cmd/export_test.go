package cmd_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
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
				Elements: []*diagv1.Element{
					{Id: 2, Name: "O1", Kind: protoPtr("service"), UpdatedAt: timestamppb.Now()},
				},
				Placements: []*diagv1.ElementPlacement{
					{ViewId: 1, ElementId: 2, PositionX: 10, PositionY: 20},
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
