package cmd_test

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"testing"

	diagv1connect "buf.build/gen/go/tldiagramcom/diagram/connectrpc/go/diag/v1/diagv1connect"
	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/workspace"
)

// ---- mock server ----

type mockDiagramService struct {
	diagv1connect.UnimplementedDiagramServiceHandler
	mu                sync.Mutex
	lastRequest       *diagv1.ApplyPlanRequest
	lastHeader        http.Header
	applyFunc         func(*diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error)
	deleteDiagramFunc func(*diagv1.DeleteDiagramRequest) (*diagv1.DeleteDiagramResponse, error)
	deleteObjectFunc  func(*diagv1.DeleteObjectRequest) (*diagv1.DeleteObjectResponse, error)
	exportFunc        func(*diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error)
}

func (m *mockDiagramService) ExportOrganization(_ context.Context, req *connect.Request[diagv1.ExportOrganizationRequest]) (*connect.Response[diagv1.ExportOrganizationResponse], error) {
	if m.exportFunc != nil {
		resp, err := m.exportFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(&diagv1.ExportOrganizationResponse{}), nil
}

func (m *mockDiagramService) ApplyPlan(_ context.Context, req *connect.Request[diagv1.ApplyPlanRequest]) (*connect.Response[diagv1.ApplyPlanResponse], error) {
	m.mu.Lock()
	m.lastRequest = req.Msg
	m.lastHeader = req.Header()
	m.mu.Unlock()

	if m.applyFunc != nil {
		resp, err := m.applyFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(successResponse(req.Msg)), nil
}

func (m *mockDiagramService) DeleteDiagram(_ context.Context, req *connect.Request[diagv1.DeleteDiagramRequest]) (*connect.Response[diagv1.DeleteDiagramResponse], error) {
	if m.deleteDiagramFunc != nil {
		resp, err := m.deleteDiagramFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(&diagv1.DeleteDiagramResponse{}), nil
}

func (m *mockDiagramService) DeleteObject(_ context.Context, req *connect.Request[diagv1.DeleteObjectRequest]) (*connect.Response[diagv1.DeleteObjectResponse], error) {
	if m.deleteObjectFunc != nil {
		resp, err := m.deleteObjectFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(&diagv1.DeleteObjectResponse{}), nil
}

func successResponse(req *diagv1.ApplyPlanRequest) *diagv1.ApplyPlanResponse {
	resp := &diagv1.ApplyPlanResponse{
		Summary: &diagv1.PlanSummary{
			DiagramsPlanned: int32(len(req.Diagrams)),
			DiagramsCreated: int32(len(req.Diagrams)),
			ObjectsPlanned:  int32(len(req.Objects)),
			ObjectsCreated:  int32(len(req.Objects)),
			EdgesPlanned:    int32(len(req.Edges)),
			EdgesCreated:    int32(len(req.Edges)),
			LinksPlanned:    int32(len(req.Links)),
			LinksCreated:    int32(len(req.Links)),
		},
		Metadata: make(map[string]*diagv1.ResourceMetadata),
	}

	var nextID int32 = 1
	for _, d := range req.Diagrams {
		id := nextID
		nextID++
		resp.CreatedDiagrams = append(resp.CreatedDiagrams, &diagv1.Diagram{
			Id:   id,
			Name: d.Name,
		})
		resp.Metadata[d.Ref] = &diagv1.ResourceMetadata{Id: id}
	}
	for _, o := range req.Objects {
		id := nextID
		nextID++
		resp.CreatedObjects = append(resp.CreatedObjects, &diagv1.Object{
			Id:   id,
			Name: o.Name,
			Type: o.Type,
		})
		resp.Metadata[o.Ref] = &diagv1.ResourceMetadata{Id: id}
	}
	for i := range req.Edges {
		id := nextID
		nextID++
		resp.CreatedEdges = append(resp.CreatedEdges, &diagv1.Edge{
			Id:             id,
			SourceObjectId: 99,  // mock source
			TargetObjectId: 100, // mock target
		})
		ref := fmt.Sprintf("edge-%d", i)
		resp.Metadata[ref] = &diagv1.ResourceMetadata{Id: id}
	}
	for i := range req.Links {
		id := nextID
		nextID++
		resp.CreatedLinks = append(resp.CreatedLinks, &diagv1.ObjectLink{
			Id:            id,
			ObjectId:      101, // mock
			FromDiagramId: 102, // mock
			ToDiagramId:   103, // mock
		})
		ref := fmt.Sprintf("link-%d", i)
		resp.Metadata[ref] = &diagv1.ResourceMetadata{Id: id}
	}
	return resp
}

func newMockServer(t *testing.T, svc *mockDiagramService) string {
	t.Helper()
	mux := http.NewServeMux()
	path, handler := diagv1connect.NewDiagramServiceHandler(svc)
	mux.Handle("/api"+path, http.StripPrefix("/api", handler))
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv.URL
}

// ---- workspace helpers ----

const testOrgID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"

// writeConfig writes a tld.yaml in TLD_CONFIG_DIR pointing at serverURL.
func writeConfig(t *testing.T, dir, serverURL, apiKey string) {
	t.Helper()
	configDir := os.Getenv("TLD_CONFIG_DIR")
	if configDir == "" {
		configDir = t.TempDir()
		t.Setenv("TLD_CONFIG_DIR", configDir)
	}
	cfg := fmt.Sprintf("server_url: %s\napi_key: %q\norg_id: %q\n", serverURL, apiKey, testOrgID)
	if err := os.MkdirAll(configDir, 0700); err != nil {
		t.Fatalf("mkdir config: %v", err)
	}
	if err := os.WriteFile(filepath.Join(configDir, "tld.yaml"), []byte(cfg), 0600); err != nil {
		t.Fatalf("write tld.yaml: %v", err)
	}
}

// setupApplyWorkspace initializes a workspace with mock server URL.
func setupApplyWorkspace(t *testing.T, dir, serverURL string) {
	t.Helper()
	// Set TLD_CONFIG_DIR once for the entire test setup
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)

	mustInitWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "test-api-key")
}

// ---- tests ----

func TestApplyCmd_SuccessAutoApprove(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}

	stdout, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}
	if !strings.Contains(stdout, "SUCCESS") {
		t.Errorf("stdout %q does not contain 'SUCCESS'", stdout)
	}
	if !strings.Contains(stdout, "## Planned vs Created") {
		t.Errorf("stdout %q does not contain summary table", stdout)
	}
}

func TestApplyCmd_BearerTokenSentToServer(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "my-secret-key")

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	authHeader := svc.lastHeader.Get("Authorization")
	svc.mu.Unlock()

	if authHeader != "Bearer my-secret-key" {
		t.Errorf("Authorization = %q, want 'Bearer my-secret-key'", authHeader)
	}
}

func TestApplyCmd_OrgIDInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "key")

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	orgID := svc.lastRequest.OrgId
	svc.mu.Unlock()

	if orgID != testOrgID {
		t.Errorf("OrgId = %q, want %q", orgID, testOrgID)
	}
}

func TestApplyCmd_DiagramsInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "diagram", "Container", "--ref", "container"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	diagrams := svc.lastRequest.Diagrams
	svc.mu.Unlock()

	if len(diagrams) != 2 {
		t.Errorf("expected 2 diagrams in request, got %d", len(diagrams))
	}
}

func TestApplyCmd_DiagramParentRefPropagated(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "Parent", "--ref", "parent"); err != nil {
		t.Fatalf("create parent: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "diagram", "Child", "--ref", "child", "--parent", "parent"); err != nil {
		t.Fatalf("create child: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	diagrams := svc.lastRequest.Diagrams
	svc.mu.Unlock()

	found := false
	for _, d := range diagrams {
		if d.Ref == "child" {
			found = true
			if d.ParentDiagramRef == nil || *d.ParentDiagramRef != "parent" {
				t.Errorf("child ParentDiagramRef = %v, want 'parent'", d.ParentDiagramRef)
			}
		}
	}
	if !found {
		t.Error("child diagram not found in request")
	}
}

func TestApplyCmd_ObjectsInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "sys", "API", "service"); err != nil {
		t.Fatalf("create object: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	objects := svc.lastRequest.Objects
	svc.mu.Unlock()

	if len(objects) != 1 {
		t.Fatalf("expected 1 object, got %d", len(objects))
	}
	if objects[0].Ref != "api" || objects[0].Name != "API" || objects[0].Type == nil || *objects[0].Type != "service" {
		t.Errorf("unexpected object: %+v", objects[0])
	}
}

func TestApplyCmd_EdgesInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "sys", "A", "service", "--ref", "a"); err != nil {
		t.Fatalf("create obj a: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "sys", "B", "service", "--ref", "b"); err != nil {
		t.Fatalf("create obj b: %v", err)
	}
	if _, _, err := runCmd(t, dir, "connect", "objects", "sys", "--from", "a", "--to", "b"); err != nil {
		t.Fatalf("connect: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	edges := svc.lastRequest.Edges
	svc.mu.Unlock()

	if len(edges) != 1 {
		t.Fatalf("expected 1 edge, got %d", len(edges))
	}
	if edges[0].DiagramRef != "sys" || edges[0].SourceObjectRef != "a" || edges[0].TargetObjectRef != "b" {
		t.Errorf("unexpected edge: %+v", edges[0])
	}
}

func TestApplyCmd_LinksInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "diagram", "Container", "--ref", "con"); err != nil {
		t.Fatalf("create container: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "sys", "API", "service", "--ref", "api"); err != nil {
		t.Fatalf("create obj: %v", err)
	}
	if _, _, err := runCmd(t, dir, "add", "link", "--object", "api", "--from", "sys", "--to", "con"); err != nil {
		t.Fatalf("add link: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	links := svc.lastRequest.Links
	svc.mu.Unlock()

	if len(links) != 1 {
		t.Fatalf("expected 1 link, got %d", len(links))
	}
	if links[0].ObjectRef != "api" || links[0].FromDiagramRef != "sys" || links[0].ToDiagramRef != "con" {
		t.Errorf("unexpected link: %+v", links[0])
	}
}

func TestApplyCmd_ServerError_CodeInternal(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(_ *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("server exploded"))
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	_, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(stderr, "Apply failed") {
		t.Errorf("stderr %q does not contain 'Apply failed'", stderr)
	}
}

func TestApplyCmd_ServerError_CodeUnauthenticated(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(_ *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			return nil, connect.NewError(connect.CodeUnauthenticated, fmt.Errorf("invalid api key"))
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	_, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil {
		t.Fatal("expected error")
	}
	if !strings.Contains(stderr, "Apply failed") {
		t.Errorf("stderr %q does not contain 'Apply failed'", stderr)
	}
}

func TestApplyCmd_DriftDetected(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			resp := successResponse(req)
			resp.Drift = []*diagv1.PlanDriftItem{
				{ResourceType: "diagram", Ref: "old", Reason: "name changed"},
			}
			return resp, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	_, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil {
		t.Fatal("expected error for drift")
	}
	if !strings.Contains(stderr, "drift item(s) detected") {
		t.Errorf("stderr %q does not contain drift warning", stderr)
	}
}

func TestApplyCmd_InteractiveApprove(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("yes\n"), "apply")
	if err != nil {
		t.Fatalf("apply with stdin yes: %v", err)
	}
	if !strings.Contains(stdout, "SUCCESS") {
		t.Errorf("stdout %q does not contain 'SUCCESS'", stdout)
	}

	svc.mu.Lock()
	called := svc.lastRequest != nil
	svc.mu.Unlock()

	if !called {
		t.Error("mock server was not called when user approved")
	}
}

func TestApplyCmd_InteractiveDecline(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("no\n"), "apply")
	if err != nil {
		t.Fatalf("apply with stdin no: %v", err)
	}
	if !strings.Contains(stdout, "Apply cancelled") {
		t.Errorf("stdout %q does not contain 'Apply cancelled'", stdout)
	}

	svc.mu.Lock()
	called := svc.lastRequest != nil
	svc.mu.Unlock()

	if called {
		t.Error("mock server should NOT be called when user declined")
	}
}

func TestApplyCmd_ValidationFailsBeforeRPC(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	mustInitWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "key")

	// Diagram with broken parent ref -> validation error
	if err := os.WriteFile(filepath.Join(dir, "diagrams.yaml"),
		[]byte("orphan: {name: Orphan, parent_diagram: nonexistent}\n"), 0600); err != nil {
		t.Fatalf("write diagrams: %v", err)
	}

	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil {
		t.Fatal("expected error for invalid workspace")
	}

	svc.mu.Lock()
	called := svc.lastRequest != nil
	svc.mu.Unlock()

	if called {
		t.Error("mock server should NOT be called when validation fails")
	}
}

func TestApplyCmd_MissingConfig(t *testing.T) {
	dir := t.TempDir()
	// No .tld.yaml
	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil {
		t.Fatal("expected error for missing config")
	}
	if !strings.Contains(err.Error(), "load workspace") {
		t.Errorf("error %q does not contain 'load workspace'", err.Error())
	}
}

func TestApplyCmd_CreatedResourcesInOutput(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	if _, _, err := runCmd(t, dir, "create", "diagram", "System", "--ref", "sys"); err != nil {
		t.Fatalf("create diagram: %v", err)
	}
	if _, _, err := runCmd(t, dir, "create", "object", "sys", "API", "service", "--ref", "api"); err != nil {
		t.Fatalf("create object: %v", err)
	}

	stdout, _, err := runCmd(t, dir, "apply", "--auto-approve", "--verbose")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}
	if !strings.Contains(stdout, "### Diagrams") {
		t.Errorf("stdout %q does not contain '### Diagrams'", stdout)
	}
	if !strings.Contains(stdout, "### Objects") {
		t.Errorf("stdout %q does not contain '### Objects'", stdout)
	}
}

func TestApplyCmd_EmptyDriftNoWarning(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			resp := successResponse(req)
			resp.Drift = []*diagv1.PlanDriftItem{} // empty
			return resp, nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)

	stdout, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}
	if strings.Contains(stderr, "drift") {
		t.Errorf("no drift warning expected, but stderr contains 'drift': %q", stderr)
	}
	if strings.Contains(stdout, "## Drift") {
		t.Errorf("no Drift section expected in stdout: %q", stdout)
	}
}

func TestApplyCmd_ConflictAbort(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			if req.DryRun != nil && *req.DryRun {
				return &diagv1.ApplyPlanResponse{
					Conflicts: []*diagv1.PlanConflictItem{
						{ResourceType: "diagram", Ref: "d1", LocalUpdatedAt: nil, RemoteUpdatedAt: nil},
					},
				}, nil
			}
			return successResponse(req), nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	// Create a lock file to trigger conflict detection
	lockFile := &workspace.LockFile{VersionID: "v1"}
	if err := workspace.WriteLockFile(dir, lockFile); err != nil {
		t.Fatal(err)
	}

	// Choice 1: Abort
	_, _, err := runCmdWithStdin(t, dir, strings.NewReader("1\n"), "apply")
	if err == nil {
		t.Fatal("expected error for abort")
	}
	if !strings.Contains(err.Error(), "apply aborted by user") {
		t.Errorf("unexpected error: %v", err)
	}
}

func TestApplyCmd_ConflictForce(t *testing.T) {
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			if req.DryRun != nil && *req.DryRun {
				return &diagv1.ApplyPlanResponse{
					Conflicts: []*diagv1.PlanConflictItem{
						{ResourceType: "diagram", Ref: "d1", LocalUpdatedAt: nil, RemoteUpdatedAt: nil},
					},
				}, nil
			}
			return successResponse(req), nil
		},
	}
	serverURL := newMockServer(t, svc)

	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	// Create a lock file to trigger conflict detection
	lockFile := &workspace.LockFile{VersionID: "v1"}
	if err := workspace.WriteLockFile(dir, lockFile); err != nil {
		t.Fatal(err)
	}

	// Choice 2: Force, then 'yes' for apply
	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("2\nyes\n"), "apply")
	if err != nil {
		t.Fatalf("apply force: %v", err)
	}
	if !strings.Contains(stdout, "SUCCESS") {
		t.Errorf("stdout %q does not contain 'SUCCESS'", stdout)
	}
}
