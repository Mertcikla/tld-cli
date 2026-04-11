package cmd_test

import (
	"context"
	"encoding/json"
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
	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
	"google.golang.org/protobuf/types/known/timestamppb"
)

type mockDiagramService struct {
	diagv1connect.UnimplementedWorkspaceServiceHandler
	mu                sync.Mutex
	lastRequest       *diagv1.ApplyPlanRequest
	lastHeader        http.Header
	applyFunc         func(*diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error)
	deleteDiagramFunc func(*diagv1.DeleteDiagramRequest) (*diagv1.DeleteDiagramResponse, error)
	deleteObjectFunc  func(*diagv1.DeleteElementRequest) (*diagv1.DeleteElementResponse, error)
	exportFunc        func(*diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error)
}

func (m *mockDiagramService) ExportWorkspace(_ context.Context, req *connect.Request[diagv1.ExportOrganizationRequest]) (*connect.Response[diagv1.ExportOrganizationResponse], error) {
	if m.exportFunc != nil {
		resp, err := m.exportFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(&diagv1.ExportOrganizationResponse{}), nil
}

func (m *mockDiagramService) ApplyWorkspacePlan(_ context.Context, req *connect.Request[diagv1.ApplyPlanRequest]) (*connect.Response[diagv1.ApplyPlanResponse], error) {
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

func (m *mockDiagramService) DeleteElement(_ context.Context, req *connect.Request[diagv1.DeleteElementRequest]) (*connect.Response[diagv1.DeleteElementResponse], error) {
	if m.deleteObjectFunc != nil {
		resp, err := m.deleteObjectFunc(req.Msg)
		if err != nil {
			return nil, err
		}
		return connect.NewResponse(resp), nil
	}
	return connect.NewResponse(&diagv1.DeleteElementResponse{}), nil
}

func successResponse(req *diagv1.ApplyPlanRequest) *diagv1.ApplyPlanResponse {
	resp := &diagv1.ApplyPlanResponse{
		Summary: &diagv1.PlanSummary{
			ElementsPlanned:   int32(len(req.Elements)),
			ElementsCreated:   int32(len(req.Elements)),
			ConnectorsPlanned: int32(len(req.Connectors)),
			ConnectorsCreated: int32(len(req.Connectors)),
		},
		ElementMetadata:   make(map[string]*diagv1.ResourceMetadata),
		DiagramMetadata:   make(map[string]*diagv1.ResourceMetadata),
		ConnectorMetadata: make(map[string]*diagv1.ResourceMetadata),
	}

	var diagramCount int32
	var nextID int32 = 1
	for _, element := range req.Elements {
		elementID := nextID
		nextID++
		resp.CreatedElements = append(resp.CreatedElements, &diagv1.Element{
			Id:           elementID,
			Name:         element.Name,
			Kind:         element.Kind,
			HasDiagram:   element.HasDiagram,
			DiagramLabel: element.DiagramLabel,
		})
		resp.ElementMetadata[element.Ref] = &diagv1.ResourceMetadata{Id: elementID, UpdatedAt: timestamppb.Now()}
		if element.HasDiagram {
			diagramID := nextID
			nextID++
			diagramCount++
			resp.CreatedDiagrams = append(resp.CreatedDiagrams, &diagv1.DiagramSummary{
				Id:             diagramID,
				OwnerElementId: &elementID,
				Name:           element.Name,
				Label:          element.DiagramLabel,
			})
			resp.DiagramMetadata[element.Ref] = &diagv1.ResourceMetadata{Id: diagramID, UpdatedAt: timestamppb.Now()}
		}
	}
	resp.Summary.DiagramsPlanned = diagramCount
	resp.Summary.DiagramsCreated = diagramCount

	for _, connector := range req.Connectors {
		connectorID := nextID
		nextID++
		resp.CreatedConnectors = append(resp.CreatedConnectors, &diagv1.Connector{
			Id:              connectorID,
			SourceElementId: 99,
			TargetElementId: 100,
			Label:           connector.Label,
			Direction:       valueOr(connector.Direction, "forward"),
			Style:           valueOr(connector.Style, "solid"),
		})
		resp.ConnectorMetadata[connector.Ref] = &diagv1.ResourceMetadata{Id: connectorID, UpdatedAt: timestamppb.Now()}
	}

	return resp
}

func valueOr(value *string, fallback string) string {
	if value == nil || *value == "" {
		return fallback
	}
	return *value
}

func newMockServer(t *testing.T, svc *mockDiagramService) string {
	t.Helper()
	mux := http.NewServeMux()
	path, handler := diagv1connect.NewWorkspaceServiceHandler(svc)
	mux.Handle("/api"+path, http.StripPrefix("/api", handler))
	srv := httptest.NewServer(mux)
	t.Cleanup(srv.Close)
	return srv.URL
}

const testOrgID = "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"

func writeConfig(t *testing.T, _, serverURL, apiKey string) {
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

func setupApplyWorkspace(t *testing.T, dir, serverURL string) {
	t.Helper()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "test-api-key")
}

func seedElementWorkspace(t *testing.T, dir string) {
	t.Helper()
	mustRunCmd(t, dir, "create", "element", "Platform", "--ref", "platform", "--kind", "workspace", "--with-view")
	mustRunCmd(t, dir, "create", "element", "API", "--ref", "api", "--parent", "platform", "--kind", "service", "--with-view")
	mustRunCmd(t, dir, "create", "element", "DB", "--ref", "db", "--parent", "platform", "--kind", "database")
	mustRunCmd(t, dir, "connect", "elements", "--view", "platform", "--from", "api", "--to", "db", "--label", "reads")
}

func TestApplyCmd_SuccessAutoApprove(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}
	if !strings.Contains(stdout, "SUCCESS") || !strings.Contains(stdout, "## Planned vs Created") {
		t.Fatalf("unexpected output: %q", stdout)
	}
}

func TestApplyCmd_BearerTokenSentToServer(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	writeConfig(t, dir, serverURL, "my-secret-key")

	if _, _, err := runCmd(t, dir, "apply", "--auto-approve"); err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	defer svc.mu.Unlock()
	if svc.lastHeader.Get("Authorization") != "Bearer my-secret-key" {
		t.Fatalf("Authorization = %q", svc.lastHeader.Get("Authorization"))
	}
}

func TestApplyCmd_OrgIDInRequest(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	if _, _, err := runCmd(t, dir, "apply", "--auto-approve"); err != nil {
		t.Fatalf("apply: %v", err)
	}

	svc.mu.Lock()
	defer svc.mu.Unlock()
	if svc.lastRequest.GetOrgId() != testOrgID {
		t.Fatalf("OrgId = %q", svc.lastRequest.GetOrgId())
	}
}

func TestApplyCmd_ElementWorkspacePersistsMetadata(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	if _, _, err := runCmd(t, dir, "apply", "--auto-approve"); err != nil {
		t.Fatalf("first apply: %v", err)
	}

	ws, err := workspace.Load(dir)
	if err != nil {
		t.Fatalf("load workspace: %v", err)
	}
	if ws.Meta == nil || ws.Meta.Elements["platform"] == nil || ws.Meta.Views["platform"] == nil || ws.Meta.Connectors["platform:api:db:reads"] == nil {
		t.Fatalf("expected workspace metadata, got %#v", ws.Meta)
	}

	if _, _, err := runCmd(t, dir, "apply", "--auto-approve"); err != nil {
		t.Fatalf("second apply: %v", err)
	}

	svc.mu.Lock()
	defer svc.mu.Unlock()
	var api *diagv1.PlanElement
	for _, element := range svc.lastRequest.Elements {
		if element.Ref == "api" {
			api = element
			break
		}
	}
	if api == nil || api.Id == nil || *api.Id == 0 || api.DiagramId == nil || *api.DiagramId == 0 {
		t.Fatalf("expected id reuse, got %#v", api)
	}
	if len(svc.lastRequest.Connectors) != 1 || svc.lastRequest.Connectors[0].Id == nil || *svc.lastRequest.Connectors[0].Id == 0 {
		t.Fatalf("expected connector id reuse, got %#v", svc.lastRequest.Connectors)
	}
}

func TestApplyCmd_ServerError_CodeInternal(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(_ *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		return nil, connect.NewError(connect.CodeInternal, fmt.Errorf("server exploded"))
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	_, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil || !strings.Contains(stderr, "Apply failed") {
		t.Fatalf("expected apply failure, stderr=%q err=%v", stderr, err)
	}
}

func TestApplyCmd_DriftDetected(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		resp := successResponse(req)
		resp.Drift = []*diagv1.PlanDriftItem{{ResourceType: "element", Ref: "api", Reason: "name changed"}}
		return resp, nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	_, stderr, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil || !strings.Contains(stderr, "drift item(s) detected") {
		t.Fatalf("expected drift failure, stderr=%q err=%v", stderr, err)
	}
}

func TestApplyCmd_PrefightDriftWarningAbort(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		if req.DryRun != nil && *req.DryRun {
			return &diagv1.ApplyPlanResponse{Drift: []*diagv1.PlanDriftItem{{ResourceType: "element", Ref: "api", Reason: "remote changed"}}}, nil
		}
		return successResponse(req), nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatal(err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1", WorkspaceHash: hash}); err != nil {
		t.Fatal(err)
	}

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("no\n"), "apply")
	if err != nil {
		t.Fatalf("expected graceful cancel, got %v", err)
	}
	if !strings.Contains(stdout, "server has changes that are not in your local YAML") || !strings.Contains(stdout, "Apply cancelled.") {
		t.Fatalf("unexpected output: %q", stdout)
	}
	if svc.lastRequest == nil || svc.lastRequest.DryRun == nil || !*svc.lastRequest.DryRun {
		t.Fatalf("expected only dry-run preflight request, got %#v", svc.lastRequest)
	}
}

func TestApplyCmd_ForceApplySkipsPreflightDriftPrompt(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		if req.DryRun != nil && *req.DryRun {
			return &diagv1.ApplyPlanResponse{Drift: []*diagv1.PlanDriftItem{{ResourceType: "element", Ref: "api", Reason: "remote changed"}}}, nil
		}
		return successResponse(req), nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatal(err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1", WorkspaceHash: hash}); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "apply", "--auto-approve", "--force-apply")
	if err != nil {
		t.Fatalf("expected apply success, stdout=%q stderr=%q err=%v", stdout, stderr, err)
	}
	if strings.Contains(stdout, "server has changes that are not in your local YAML") {
		t.Fatalf("unexpected preflight prompt output: %q", stdout)
	}
}

func TestApplyCmd_InteractiveApprove(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("yes\n"), "apply")
	if err != nil || !strings.Contains(stdout, "SUCCESS") {
		t.Fatalf("stdout=%q err=%v", stdout, err)
	}
}

func TestApplyCmd_InteractiveDecline(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("no\n"), "apply")
	if err != nil {
		t.Fatalf("apply with stdin no: %v", err)
	}
	if !strings.Contains(stdout, "Apply cancelled") {
		t.Fatalf("stdout=%q", stdout)
	}
}

func TestApplyCmd_MissingConfig(t *testing.T) {
	dir := t.TempDir()
	_, _, err := runCmd(t, dir, "apply", "--auto-approve")
	if err == nil || !strings.Contains(err.Error(), "load workspace") {
		t.Fatalf("expected missing config error, got %v", err)
	}
}

func TestApplyCmd_CreatedResourcesInOutput(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, _, err := runCmd(t, dir, "apply", "--auto-approve", "--verbose")
	if err != nil {
		t.Fatalf("apply: %v", err)
	}
	if !strings.Contains(stdout, "### Diagrams") || !strings.Contains(stdout, "### Elements") || !strings.Contains(stdout, "### Connectors") {
		t.Fatalf("unexpected verbose output: %q", stdout)
	}
}

func TestApplyCmd_ConflictAbort(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		if req.DryRun != nil && *req.DryRun {
			return &diagv1.ApplyPlanResponse{Conflicts: []*diagv1.PlanConflictItem{{ResourceType: "element", Ref: "api"}}}, nil
		}
		return successResponse(req), nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1"}); err != nil {
		t.Fatal(err)
	}

	_, _, err := runCmdWithStdin(t, dir, strings.NewReader("1\n"), "apply")
	if err == nil || !strings.Contains(err.Error(), "apply aborted by user") {
		t.Fatalf("expected abort error, got %v", err)
	}
}

func TestApplyCmd_ConflictForce(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		if req.DryRun != nil && *req.DryRun {
			return &diagv1.ApplyPlanResponse{Conflicts: []*diagv1.PlanConflictItem{{ResourceType: "element", Ref: "api"}}}, nil
		}
		return successResponse(req), nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1"}); err != nil {
		t.Fatal(err)
	}

	stdout, _, err := runCmdWithStdin(t, dir, strings.NewReader("2\nyes\n"), "apply")
	if err != nil || !strings.Contains(stdout, "SUCCESS") {
		t.Fatalf("stdout=%q err=%v", stdout, err)
	}
}

func TestApplyCmd_JSONOutput(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)

	stdout, stderr, err := runCmd(t, dir, "apply", "--auto-approve", "--format", "json")
	if err != nil {
		t.Fatalf("apply --format json: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	var payload planner.JSONOutput
	if err := json.Unmarshal([]byte(stdout), &payload); err != nil {
		t.Fatalf("unmarshal json output: %v\nstdout=%s", err, stdout)
	}
	if payload.Command != "apply" || payload.Status != "ok" {
		t.Fatalf("unexpected payload: %+v", payload)
	}
	if payload.Retries != 0 {
		t.Fatalf("unexpected retries: %+v", payload)
	}
}

func TestApplyCmd_JSONOutputIncludesRetryCount(t *testing.T) {
	var calls int
	svc := &mockDiagramService{
		applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
			calls++
			if req.DryRun != nil && *req.DryRun {
				return &diagv1.ApplyPlanResponse{Conflicts: []*diagv1.PlanConflictItem{{ResourceType: "element", Ref: "api"}}}, nil
			}
			return successResponse(req), nil
		},
		exportFunc: func(_ *diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error) {
			return &diagv1.ExportOrganizationResponse{}, nil
		},
	}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1"}); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "apply", "--auto-approve", "--format", "json")
	if err != nil {
		t.Fatalf("apply --format json: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	var payload planner.JSONOutput
	if err := json.Unmarshal([]byte(stdout), &payload); err != nil {
		t.Fatalf("unmarshal json output: %v\nstdout=%s", err, stdout)
	}
	if payload.Retries != 1 {
		t.Fatalf("expected retries=1, got %+v", payload)
	}
	if calls < 2 {
		t.Fatalf("expected dry-run conflict check and real apply, got %d calls", calls)
	}
}
