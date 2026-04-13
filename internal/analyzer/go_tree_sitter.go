package analyzer

import (
	"context"
	"fmt"
	"strings"

	sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_go "github.com/tree-sitter/tree-sitter-go/bindings/go"
)

type goParser struct{}

func (p *goParser) ParseFile(ctx context.Context, path string, source []byte) (*Result, error) {
	parser := sitter.NewParser()
	defer parser.Close()

	if err := parser.SetLanguage(sitter.NewLanguage(tree_sitter_go.Language())); err != nil {
		return nil, fmt.Errorf("set go tree-sitter language: %w", err)
	}

	tree := parser.ParseCtx(ctx, source, nil)
	defer tree.Close()

	result := &Result{}
	root := tree.RootNode()
	p.walkNode(root, source, path, result)
	return result, nil
}

func (p *goParser) walkNode(node *sitter.Node, source []byte, path string, result *Result) {
	if node == nil {
		return
	}

	switch node.Kind() {
	case "function_declaration":
		p.appendFunction(node, source, path, "function", result)
	case "method_declaration":
		p.appendFunction(node, source, path, "method", result)
	case "type_spec":
		p.appendTypeSpec(node, source, path, result)
	case "type_alias":
		p.appendTypeAlias(node, source, path, result)
	case "call_expression":
		p.appendCall(node, source, path, result)
	}

	cursor := node.Walk()
	defer cursor.Close()
	for _, child := range node.NamedChildren(cursor) {
		childCopy := child
		p.walkNode(&childCopy, source, path, result)
	}
}

func (p *goParser) appendFunction(node *sitter.Node, source []byte, path, kind string, result *Result) {
	nameNode := node.ChildByFieldName("name")
	if nameNode == nil {
		return
	}
	result.Symbols = append(result.Symbols, Symbol{
		Name:     nameNode.Utf8Text(source),
		Kind:     kind,
		FilePath: path,
		Line:     int(nameNode.StartPosition().Row) + 1,
		EndLine:  int(node.EndPosition().Row) + 1,
	})
}

func (p *goParser) appendTypeSpec(node *sitter.Node, source []byte, path string, result *Result) {
	nameNode := node.ChildByFieldName("name")
	if nameNode == nil {
		return
	}
	typeNode := node.ChildByFieldName("type")
	kind := "type"
	if typeNode != nil {
		switch typeNode.Kind() {
		case "struct_type":
			kind = "struct"
		case "interface_type":
			kind = "interface"
		}
	}
	result.Symbols = append(result.Symbols, Symbol{
		Name:     nameNode.Utf8Text(source),
		Kind:     kind,
		FilePath: path,
		Line:     int(nameNode.StartPosition().Row) + 1,
		EndLine:  int(node.EndPosition().Row) + 1,
	})
}

func (p *goParser) appendTypeAlias(node *sitter.Node, source []byte, path string, result *Result) {
	nameNode := node.ChildByFieldName("name")
	if nameNode == nil {
		return
	}
	result.Symbols = append(result.Symbols, Symbol{
		Name:     nameNode.Utf8Text(source),
		Kind:     "type",
		FilePath: path,
		Line:     int(nameNode.StartPosition().Row) + 1,
		EndLine:  int(node.EndPosition().Row) + 1,
	})
}

func (p *goParser) appendCall(node *sitter.Node, source []byte, path string, result *Result) {
	functionNode := node.ChildByFieldName("function")
	if functionNode == nil {
		return
	}
	name := goCallName(functionNode, source)
	if name == "" {
		return
	}
	result.Refs = append(result.Refs, Ref{
		Name:     name,
		FilePath: path,
		Line:     int(functionNode.StartPosition().Row) + 1,
		Column:   int(functionNode.StartPosition().Column) + 1,
	})
}

func goCallName(node *sitter.Node, source []byte) string {
	if node == nil {
		return ""
	}
	switch node.Kind() {
	case "identifier", "field_identifier", "type_identifier":
		return node.Utf8Text(source)
	case "selector_expression":
		fieldNode := node.ChildByFieldName("field")
		if fieldNode != nil {
			return fieldNode.Utf8Text(source)
		}
	case "parenthesized_expression":
		cursor := node.Walk()
		defer cursor.Close()
		children := node.NamedChildren(cursor)
		if len(children) > 0 {
			child := children[0]
			return goCallName(&child, source)
		}
	}
	text := strings.TrimSpace(node.Utf8Text(source))
	if text == "" {
		return ""
	}
	if index := strings.LastIndex(text, "."); index >= 0 {
		text = text[index+1:]
	}
	if index := strings.Index(text, "["); index >= 0 {
		text = text[:index]
	}
	return strings.TrimSpace(text)
}
