package cmd

import (
	"path/filepath"
	"testing"

	"github.com/mertcikla/tld-cli/workspace"
)

func TestEnsureAnalyzeElement_ReusesIdentityWithinRun(t *testing.T) {
	dir := t.TempDir()
	ws := &workspace.Workspace{
		Dir:      dir,
		Elements: map[string]*workspace.Element{},
	}
	known := buildAnalyzeElementIndex(ws)
	knownNames := buildAnalyzeElementNameIndex(ws)
	usedRefs := map[string]struct{}{}
	spec := analyzeElementSpec{
		Name:      "alias_for_table",
		Kind:      "function",
		Owner:     "digitaltwin-poc",
		Repo:      "git@gitlab.btsgrp.com:ridvan.zengin/dtwin.git",
		Branch:    "main",
		FilePath:  filepath.Clean("backhaul_analysis/backhaul_analysis.py"),
		Symbol:    "alias_for_table",
		ParentRef: "backhaul-analysis-py",
		Identity: analyzeElementIdentity{
			Repo:     "git@gitlab.btsgrp.com:ridvan.zengin/dtwin.git",
			Branch:   "main",
			FilePath: "backhaul_analysis/backhaul_analysis.py",
			Symbol:   "alias_for_table",
			Kind:     "function",
			Name:     "alias_for_table",
		},
	}

	firstRef, err := ensureAnalyzeElement(dir, false, ws, known, knownNames, usedRefs, spec)
	if err != nil {
		t.Fatalf("first ensureAnalyzeElement: %v", err)
	}
	secondRef, err := ensureAnalyzeElement(dir, false, ws, known, knownNames, usedRefs, spec)
	if err != nil {
		t.Fatalf("second ensureAnalyzeElement: %v", err)
	}
	if firstRef != secondRef {
		t.Fatalf("refs differ: first=%q second=%q", firstRef, secondRef)
	}
	if len(ws.Elements) != 1 {
		t.Fatalf("elements = %d, want 1", len(ws.Elements))
	}
	if _, ok := ws.Elements[firstRef]; !ok {
		t.Fatalf("missing expected ref %q in ws.Elements", firstRef)
	}
}
