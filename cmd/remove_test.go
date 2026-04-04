package cmd_test

import (
	"strings"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
)

func TestRemoveDiagramCmd_LocalOnly(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	// Create a diagram first
	_, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys")
	if err != nil {
		t.Fatalf("create diagram: %v", err)
	}

	// Remove it
	stdout, _, err := runCmd(t, dir, "remove", "diagram", "sys", "--offline")
	if err != nil {
		t.Fatalf("remove diagram: %v", err)
	}
	if !strings.Contains(stdout, "Removed sys from diagrams.yaml") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}

func TestRemoveDiagramCmd_Server(t *testing.T) {
	var deletedID int32
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			resp := successResponse(req)
			if len(resp.CreatedDiagrams) > 0 {
				resp.CreatedDiagrams[0].Id = 100
			}
			resp.Metadata["sys"] = &diagv1.ResourceMetadata{Id: 100}
			return resp, nil
		},
		deleteDiagramFunc: func(req *diagv1.DeleteDiagramRequest) (*diagv1.DeleteDiagramResponse, error) {
			deletedID = req.DiagramId
			return &diagv1.DeleteDiagramResponse{}, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	// Create and apply to get a server ID
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "apply", "--auto-approve"); err != nil {
		t.Fatalf("apply: %v", err)
	}

	// Remove it (without --offline)
	stdout, _, err := runCmd(t, dir, "remove", "diagram", "sys")
	if err != nil {
		t.Fatalf("remove diagram: %v", err)
	}
	if !strings.Contains(stdout, "deleted from server (id=100)") {
		t.Errorf("stdout %q does not contain server deletion message", stdout)
	}
	if deletedID != 100 {
		t.Errorf("expected deleted ID 100, got %d", deletedID)
	}
}

func TestRemoveObjectCmd_LocalOnly(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	if _, _, err := runCmd(t, dir, "create", "diagram", "D1", "--ref", "d1"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "d1", "Obj1", "service", "--ref", "o1"); err != nil {
		t.Fatalf("create object: %v", err)
	}

	stdout, _, err := runCmd(t, dir, "remove", "object", "o1", "--offline")
	if err != nil {
		t.Fatalf("remove object: %v", err)
	}
	if !strings.Contains(stdout, "Removed o1 from objects.yaml") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}

func TestRemoveObjectCmd_Server(t *testing.T) {
	var deletedID int32
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			resp := successResponse(req)
			if len(resp.CreatedObjects) > 0 {
				resp.CreatedObjects[0].Id = 200
			}
			resp.Metadata["o1"] = &diagv1.ResourceMetadata{Id: 200}
			return resp, nil
		},
		deleteObjectFunc: func(req *diagv1.DeleteObjectRequest) (*diagv1.DeleteObjectResponse, error) {
			deletedID = req.ObjectId
			return &diagv1.DeleteObjectResponse{}, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	// Create and apply to get a server ID
	mustRunCmd(t, dir, "create", "diagram", "D1", "--ref", "d1")
	mustRunCmd(t, dir, "create", "object", "d1", "Obj1", "service", "--ref", "o1")
	mustRunCmd(t, dir, "apply", "--auto-approve")

	// Remove it (without --offline)
	stdout, _, err := runCmd(t, dir, "remove", "object", "o1")
	if err != nil {
		t.Fatalf("remove object: %v", err)
	}
	if !strings.Contains(stdout, "deleted from server (id=200)") {
		t.Errorf("stdout %q does not contain server deletion message", stdout)
	}
	if deletedID != 200 {
		t.Errorf("expected deleted ID 200, got %d", deletedID)
	}
}

func TestRemoveEdgeCmd(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	mustRunCmd(t, dir, "create", "diagram", "D1", "--ref", "d1")
	mustRunCmd(t, dir, "create", "object", "d1", "A", "service", "--ref", "a")
	mustRunCmd(t, dir, "create", "object", "d1", "B", "service", "--ref", "b")
	mustRunCmd(t, dir, "connect", "objects", "d1", "--from", "a", "--to", "b")

	stdout, _, err := runCmd(t, dir, "remove", "edge", "--diagram", "d1", "--from", "a", "--to", "b")
	if err != nil {
		t.Fatalf("remove edge: %v", err)
	}
	if !strings.Contains(stdout, "Removed 1 edge(s)") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}

func TestRemoveLinkCmd(t *testing.T) {
	dir := t.TempDir()
	mustInitWorkspace(t, dir)

	mustRunCmd(t, dir, "create", "diagram", "D1", "--ref", "d1")
	mustRunCmd(t, dir, "create", "diagram", "D2", "--ref", "d2")
	mustRunCmd(t, dir, "create", "object", "d1", "A", "service", "--ref", "a")
	mustRunCmd(t, dir, "create", "link", "--object", "a", "--from", "d1", "--to", "d2")

	stdout, _, err := runCmd(t, dir, "remove", "link", "--object", "a", "--from", "d1", "--to", "d2")
	if err != nil {
		t.Fatalf("remove link: %v", err)
	}
	if !strings.Contains(stdout, "Removed 1 link(s)") {
		t.Errorf("stdout %q does not contain success message", stdout)
	}
}
