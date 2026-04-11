package ignore

import "testing"

func TestShouldIgnoreRepo(t *testing.T) {
	r := &Rules{Repos: []string{"https://github.com/org/private-repo"}}
	if !r.ShouldIgnoreRepo("https://github.com/org/private-repo") {
		t.Error("expected exact repo match")
	}
	if r.ShouldIgnoreRepo("https://github.com/org/other-repo") {
		t.Error("should not match different repo")
	}
}

func TestShouldIgnoreFolder(t *testing.T) {
	r := &Rules{Folders: []string{"vendor/", "node_modules/", ".git/"}}
	cases := []struct {
		path   string
		expect bool
	}{
		{"vendor", true},
		{"vendor/foo", true},
		{"node_modules", true},
		{"node_modules/lodash", true},
		{".git", true},
		{"src/vendor/lib", false}, // vendor only at root segment
		{"myvendor", false},
		{"src", false},
	}
	for _, c := range cases {
		got := r.ShouldIgnoreFolder(c.path)
		if got != c.expect {
			t.Errorf("ShouldIgnoreFolder(%q) = %v, want %v", c.path, got, c.expect)
		}
	}
}

func TestShouldIgnoreFile(t *testing.T) {
	r := &Rules{Files: []string{"*_test.go", "*.pb.go"}}
	cases := []struct {
		path   string
		expect bool
	}{
		{"foo_test.go", true},
		{"handler_test.go", true},
		{"handler.go", false},
		{"service.pb.go", true},
		{"main.go", false},
		{"src/foo_test.go", true},
	}
	for _, c := range cases {
		got := r.ShouldIgnoreFile(c.path)
		if got != c.expect {
			t.Errorf("ShouldIgnoreFile(%q) = %v, want %v", c.path, got, c.expect)
		}
	}
}

func TestShouldIgnoreSymbol(t *testing.T) {
	r := &Rules{Symbols: []string{"internal*", "test*", "TestMain"}}
	cases := []struct {
		name   string
		expect bool
	}{
		{"internalHelper", true},
		{"testSetup", true},
		{"TestMain", true},
		{"MyPublicFunc", false},
		{"HandleRequest", false},
	}
	for _, c := range cases {
		got := r.ShouldIgnoreSymbol(c.name)
		if got != c.expect {
			t.Errorf("ShouldIgnoreSymbol(%q) = %v, want %v", c.name, got, c.expect)
		}
	}
}

func TestNilRules(t *testing.T) {
	var r *Rules
	if r.ShouldIgnoreRepo("anything") {
		t.Error("nil rules should never ignore")
	}
	if r.ShouldIgnoreFolder("vendor") {
		t.Error("nil rules should never ignore")
	}
	if r.ShouldIgnoreFile("foo.go") {
		t.Error("nil rules should never ignore")
	}
	if r.ShouldIgnoreSymbol("foo") {
		t.Error("nil rules should never ignore")
	}
}
