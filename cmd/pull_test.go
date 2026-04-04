package cmd_test

import (
	"os"
	"path/filepath"
	"testing"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"google.golang.org/protobuf/types/known/timestamppb"
	"gopkg.in/yaml.v3"
)

func TestPullCmd_PreserveRefs(t *testing.T) {
	// 1. Setup workspace with a custom ref
	dir := t.TempDir()

	// Set TLD_CONFIG_DIR to use our temp dir for tld.yaml
	t.Setenv("TLD_CONFIG_DIR", dir)

	// Create tld.yaml in the temp dir
	err := os.WriteFile(filepath.Join(dir, "tld.yaml"), []byte("server_url: http://localhost\napi_key: test\norg_id: test-org\n"), 0600)
	if err != nil {
		t.Fatal(err)
	}

	// Create objects.yaml with a custom ref "my-custom-ref" for object named "API Service"
	// ID "5nKqb4Xa" decodes to 22 in tld-cli hashids
	objectsYAML := `
my-custom-ref:
  name: API Service
  type: service
_meta:
  my-custom-ref:
    id: "5nKqb4Xa"
    updated_at: "2023-01-01T00:00:00Z"
`
	err = os.WriteFile(filepath.Join(dir, "objects.yaml"), []byte(objectsYAML), 0600)
	if err != nil {
		t.Fatal(err)
	}

	// 2. Setup mock server that returns "API Service" with ID matching "5nKqb4Xa"
	svc := &mockDiagramService{
		exportFunc: func(_ *diagv1.ExportOrganizationRequest) (*diagv1.ExportOrganizationResponse, error) {
			resp := &diagv1.ExportOrganizationResponse{
				Objects: []*diagv1.Object{
					{
						Id:          22, // This matches 5nKqb4Xa
						Name:        "API Service Updated",
						Type:        protoPtr("service"),
						UpdatedAt:   timestamppb.Now(),
						Description: protoPtr("New description"),
					},
				},
			}
			return resp, nil
		},
	}

	serverURL := newMockServer(t, svc)
	// Update config with mock server URL
	err = os.WriteFile(filepath.Join(dir, "tld.yaml"), []byte("server_url: "+serverURL+"\napi_key: test\norg_id: test-org\n"), 0600)
	if err != nil {
		t.Fatal(err)
	}

	// 3. Run pull
	_, _, err = runCmd(t, dir, "pull", "--force")
	if err != nil {
		t.Fatalf("pull: %v", err)
	}

	// 4. Verify objects.yaml still uses "my-custom-ref"
	data, err := os.ReadFile(filepath.Join(dir, "objects.yaml"))
	if err != nil {
		t.Fatal(err)
	}

	var result map[string]any
	if err := yaml.Unmarshal(data, &result); err != nil {
		t.Fatal(err)
	}

	if _, ok := result["my-custom-ref"]; !ok {
		t.Errorf("objects.yaml lost custom ref 'my-custom-ref'. Keys: %v", getMapKeys(result))
	}

	obj := result["my-custom-ref"].(map[string]any)
	if obj["name"] != "API Service Updated" {
		t.Errorf("Expected name 'API Service Updated', got %q", obj["name"])
	}
	if obj["description"] != "New description" {
		t.Errorf("Expected description 'New description', got %q", obj["description"])
	}
}

func getMapKeys(m map[string]any) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return keys
}
