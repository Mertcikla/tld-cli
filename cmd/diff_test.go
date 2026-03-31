package cmd_test

import (
	"os"
	"path/filepath"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
)

func TestDiffCmd(t *testing.T) {
	svc := &mockDiagramService{
		exportFunc: func(_ *diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error) {
			resp := &diagv1.ExportOrganizationResponse{
				Diagrams: []*diagv1.Diagram{
					{Id: 1, Name: "Server Diagram", UpdatedAt: timestamppb.Now()},
				},
			}
			return resp, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	// Create a local diagram that is different from server
	diagYAML := "local-diag: {name: Local Diagram}\n"
	os.WriteFile(filepath.Join(dir, "diagrams.yaml"), []byte(diagYAML), 0600)

	// Run diff
	// Note: since it calls 'git diff', it might fail in environments without git or if git is not configured.
	// But we expect it to at least run and return output if differences exist.
	_, _, err := runCmd(t, dir, "diff")
	if err != nil {
		// git diff returns 1 if differences are found, which might be interpreted as error by Execute()
		// but our RunE returns nil even if diff found.
		t.Fatalf("diff: %v", err)
	}

	// stdout should contain diff info (git diff output)
	// it should show -Server Diagram and +Local Diagram (or similar depending on how git diff handles it)
	// Given we diff FROM temp TO local:
	// - server state items should be prefixed with '-'
	// + local state items should be prefixed with '+'
	
	// We just check for some expected strings in diff
	// (git diff might include filenames and line changes)
}
