package cmd_test

import (
	"encoding/json"
	"strings"
	"testing"
	"time"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/workspace"
)

func TestStatusCmd_Clean(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)

	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatalf("CalculateWorkspaceHash: %v", err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{
		Version:       "v1",
		VersionID:     "version-1",
		LastApply:     time.Now(),
		AppliedBy:     "cli",
		Resources:     &workspace.ResourceCounts{},
		WorkspaceHash: hash,
	}); err != nil {
		t.Fatalf("WriteLockFile: %v", err)
	}

	stdout, stderr, err := runCmd(t, dir, "status")
	if err != nil {
		t.Fatalf("status: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "IN SYNC") {
		t.Fatalf("missing IN SYNC header:\n%s", stdout)
	}
	if !strings.Contains(stdout, "Local changes: Clean") {
		t.Fatalf("missing clean status detail:\n%s", stdout)
	}
}

func TestStatusCmd_Modified(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)

	if err := workspace.WriteLockFile(dir, &workspace.LockFile{
		Version:       "v1",
		VersionID:     "version-1",
		LastApply:     time.Now(),
		AppliedBy:     "cli",
		Resources:     &workspace.ResourceCounts{},
		WorkspaceHash: "sha256:stale",
	}); err != nil {
		t.Fatalf("WriteLockFile: %v", err)
	}

	stdout, stderr, err := runCmd(t, dir, "status")
	if err != nil {
		t.Fatalf("status: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "MODIFIED") {
		t.Fatalf("missing MODIFIED header:\n%s", stdout)
	}
	if !strings.Contains(stdout, "Local changes: Modified") {
		t.Fatalf("missing modified detail:\n%s", stdout)
	}
}

func TestStatusCmd_NoLockFile(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)

	stdout, stderr, err := runCmd(t, dir, "status")
	if err != nil {
		t.Fatalf("status: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "No sync history found.") {
		t.Fatalf("missing no-lock message:\n%s", stdout)
	}
}

func TestStatusCmd_JSONOutput(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)

	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatalf("CalculateWorkspaceHash: %v", err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{
		Version:       "v1",
		VersionID:     "version-1",
		LastApply:     time.Now(),
		AppliedBy:     "cli",
		Resources:     &workspace.ResourceCounts{},
		WorkspaceHash: hash,
	}); err != nil {
		t.Fatalf("WriteLockFile: %v", err)
	}

	stdout, stderr, err := runCmd(t, dir, "status", "--format", "json")
	if err != nil {
		t.Fatalf("status --format json: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	var payload planner.JSONOutput
	if err := json.Unmarshal([]byte(stdout), &payload); err != nil {
		t.Fatalf("unmarshal json output: %v\nstdout=%s", err, stdout)
	}
	if payload.Command != "status" || payload.Status != "in_sync" {
		t.Fatalf("unexpected payload: %+v", payload)
	}
}

func TestStatusCmd_ConflictCount(t *testing.T) {
	dir := t.TempDir()
	configDir := t.TempDir()
	t.Setenv("TLD_CONFIG_DIR", configDir)
	mustInitWorkspace(t, dir)

	if err := workspace.WriteMetadataSection(dir, "elements.yaml", "_meta_elements", map[string]*workspace.ResourceMetadata{
		"api": {Conflict: true},
		"db":  {Conflict: true},
	}); err != nil {
		t.Fatalf("WriteMetadataSection: %v", err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "version-1", LastApply: time.Now()}); err != nil {
		t.Fatalf("WriteLockFile: %v", err)
	}

	stdout, stderr, err := runCmd(t, dir, "status")
	if err != nil {
		t.Fatalf("status: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "Merge conflicts: 2") {
		t.Fatalf("missing conflict count: %s", stdout)
	}
}

func TestStatusCmd_CheckServer_InSync(t *testing.T) {
	svc := &mockDiagramService{}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatal(err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1", WorkspaceHash: hash, LastApply: time.Now()}); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "status", "--check-server")
	if err != nil {
		t.Fatalf("status --check-server: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "Server state:  In sync") {
		t.Fatalf("missing in-sync server output: %s", stdout)
	}
}

func TestStatusCmd_CheckServer_Drifted(t *testing.T) {
	svc := &mockDiagramService{applyFunc: func(req *diagv1.ApplyPlanRequest) (*diagv1.ApplyPlanResponse, error) {
		resp := successResponse(req)
		resp.Drift = []*diagv1.PlanDriftItem{{ResourceType: "element", Ref: "api", Reason: "server changed"}}
		return resp, nil
	}}
	serverURL := newMockServer(t, svc)
	dir := t.TempDir()
	setupApplyWorkspace(t, dir, serverURL)
	seedElementWorkspace(t, dir)
	hash, err := workspace.CalculateWorkspaceHash(dir)
	if err != nil {
		t.Fatal(err)
	}
	if err := workspace.WriteLockFile(dir, &workspace.LockFile{VersionID: "v1", WorkspaceHash: hash, LastApply: time.Now()}); err != nil {
		t.Fatal(err)
	}

	stdout, stderr, err := runCmd(t, dir, "status", "--check-server")
	if err != nil {
		t.Fatalf("status --check-server: %v\nstdout: %s\nstderr: %s", err, stdout, stderr)
	}
	if !strings.Contains(stdout, "1 drift items found") {
		t.Fatalf("missing drift output: %s", stdout)
	}
}
