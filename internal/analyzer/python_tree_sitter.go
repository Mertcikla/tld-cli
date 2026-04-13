package analyzer

import (
	"context"
	"fmt"
	"strings"

	sitter "github.com/tree-sitter/go-tree-sitter"
	tree_sitter_python "github.com/tree-sitter/tree-sitter-python/bindings/go"
)

type pythonParser struct{}

func (p *pythonParser) ParseFile(ctx context.Context, path string, source []byte) (*Result, error) {
	parser := sitter.NewParser()
	defer parser.Close()

	if err := parser.SetLanguage(sitter.NewLanguage(tree_sitter_python.Language())); err != nil {
		return nil, fmt.Errorf("set python tree-sitter language: %w", err)
	}

	tree := parser.ParseCtx(ctx, source, nil)
	defer tree.Close()

	result := &Result{}
	root := tree.RootNode()
	p.walkNode(root, source, path, result)
	return result, nil
}

func (p *pythonParser) walkNode(node *sitter.Node, source []byte, path string, result *Result) {
	if node == nil {
		return
	}

	switch node.Kind() {
	case "function_definition":
		p.appendSymbol(node, source, path, "function", result)
	case "class_definition":
		p.appendSymbol(node, source, path, "class", result)
	case "call":
		p.appendCall(node, source, path, result)
	}

	cursor := node.Walk()
	defer cursor.Close()
	for _, child := range node.NamedChildren(cursor) {
		childCopy := child
		p.walkNode(&childCopy, source, path, result)
	}
}

func (p *pythonParser) appendSymbol(node *sitter.Node, source []byte, path, kind string, result *Result) {
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

func (p *pythonParser) appendCall(node *sitter.Node, source []byte, path string, result *Result) {
	functionNode := node.ChildByFieldName("function")
	if functionNode == nil {
		return
	}
	name := pythonCallName(functionNode, source)
	if name == "" {
		return
	}
	result.Refs = append(result.Refs, Ref{
		Name:     name,
		FilePath: path,
		Line:     int(functionNode.StartPosition().Row) + 1,
	})
}

func pythonCallName(node *sitter.Node, source []byte) string {
	if node == nil {
		return ""
	}
	switch node.Kind() {
	case "identifier":
		return node.Utf8Text(source)
	case "attribute":
		attributeNode := node.ChildByFieldName("attribute")
		if attributeNode != nil {
			return attributeNode.Utf8Text(source)
		}
	case "call":
		callNode := node.ChildByFieldName("function")
		if callNode != nil {
			return pythonCallName(callNode, source)
		}
	}
	text := strings.TrimSpace(node.Utf8Text(source))
	if text == "" {
		return ""
	}
	if index := strings.LastIndex(text, "."); index >= 0 {
		text = text[index+1:]
	}
	if index := strings.Index(text, "("); index >= 0 {
		text = text[:index]
	}
	return strings.TrimSpace(text)
}
