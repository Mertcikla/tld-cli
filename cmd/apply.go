package cmd

import (
	"bufio"

	"errors"
	"fmt"
	"strings"
	"time"

	diagv1 "buf.build/gen/go/tldiagramcom/diagram/protocolbuffers/go/diag/v1"
	"connectrpc.com/connect"
	"github.com/mertcikla/tld-cli/client"
	"github.com/mertcikla/tld-cli/planner"
	"github.com/mertcikla/tld-cli/reporter"
	"github.com/mertcikla/tld-cli/workspace"
	"github.com/spf13/cobra"
	"google.golang.org/protobuf/proto"
)

func newApplyCmd(wdir *string) *cobra.Command {
	var autoApprove bool
	var debug bool
	var verbose bool
	var recreateIDs bool

	c := &cobra.Command{
		Use:   "apply",
		Short: "Apply plan to the tldiagram.com",
		RunE: func(cmd *cobra.Command, _ []string) error {
			ws, err := workspace.Load(*wdir)
			if err != nil {
				return fmt.Errorf("load workspace: %w", err)
			}
			if errs := ws.Validate(); len(errs) > 0 {
				for _, e := range errs {
					fmt.Fprintf(cmd.ErrOrStderr(), "validation error: %s\n", e)
				}
				return fmt.Errorf("workspace has %d validation error(s)", len(errs))
			}

			// Load lock file and metadata for conflict detection
			lockFile, err := workspace.LoadLockFile(*wdir)
			if err != nil {
				return fmt.Errorf("load lock file: %w", err)
			}

			meta, err := workspace.LoadMetadata(*wdir)
			if err != nil {
				return fmt.Errorf("load metadata: %w", err)
			}

			plan, err := planner.Build(ws, recreateIDs)
			if err != nil {
				return fmt.Errorf("build plan: %w", err)
			}

			req := plan.Request
			diagramCount := 0
			for _, element := range req.Elements {
				if element.GetHasDiagram() {
					diagramCount++
				}
			}
			total := len(req.Elements) + diagramCount + len(req.Connectors)
			fmt.Fprintf(cmd.OutOrStdout(), "Plan: %d elements, %d diagrams, %d connectors (%d total resources)\n",
				len(req.Elements), diagramCount, len(req.Connectors), total)

			// Check for version conflicts if lock file exists
			scanner := bufio.NewScanner(cmd.InOrStdin())
			if lockFile != nil && !autoApprove {
				if err := detectAndHandleConflicts(cmd, ws, lockFile, meta, plan, scanner); err != nil {
					return err
				}
			}

			if !autoApprove {
				fmt.Fprintf(cmd.OutOrStdout(), "Apply %d resources? [yes/no]: ", total)
				if !scanner.Scan() {
					return errors.New("aborted")
				}
				answer := strings.TrimSpace(strings.ToLower(scanner.Text()))
				if answer != "yes" && answer != "y" {
					fmt.Fprintln(cmd.OutOrStdout(), "Apply cancelled.")
					return nil
				}
			}

			c := client.New(ws.Config.ServerURL, ws.Config.APIKey, debug)
			resp, err := c.ApplyWorkspacePlan(cmd.Context(), connect.NewRequest(req))
			if err != nil {
				fmt.Fprintln(cmd.ErrOrStderr(), "Apply failed:", err)
				fmt.Fprintf(cmd.ErrOrStderr(), "  Target URL: %s\n", client.NormalizeURL(ws.Config.ServerURL))

				if connectErr := new(connect.Error); errors.As(err, &connectErr) {
					fmt.Fprintf(cmd.ErrOrStderr(), "  Code: %s\n", connectErr.Code().String())
					if len(connectErr.Details()) > 0 {
						fmt.Fprintln(cmd.ErrOrStderr(), "  Details:")
						for _, detail := range connectErr.Details() {
							fmt.Fprintf(cmd.ErrOrStderr(), "    - %v\n", detail)
						}
					}
				}

				fmt.Fprintln(cmd.ErrOrStderr(), "Transaction rolled back.")
				reporter.RenderExecutionMarkdown(cmd.ErrOrStderr(), plan, nil, false, false)
				return fmt.Errorf("apply failed: %w", err)
			}

			if err := updatePlanMetadataFromResponse(*wdir, meta, ws, plan, resp.Msg); err != nil {
				fmt.Fprintf(cmd.ErrOrStderr(), "Warning: failed to update metadata: %v\n", err)
			}

			if err := updateLockFileFromResponse(*wdir, lockFile, ws, resp.Msg); err != nil {
				fmt.Fprintf(cmd.ErrOrStderr(), "Warning: failed to update lock file: %v\n", err)
			}

			reporter.RenderExecutionMarkdown(cmd.OutOrStdout(), plan, resp.Msg, true, verbose)

			if len(resp.Msg.Drift) > 0 {
				fmt.Fprintf(cmd.ErrOrStderr(), "Warning: %d drift item(s) detected\n", len(resp.Msg.Drift))
				return fmt.Errorf("%d drift item(s) detected", len(resp.Msg.Drift))
			}
			return nil
		},
	}

	c.Flags().BoolVar(&autoApprove, "auto-approve", false, "skip interactive approval prompt")
	c.Flags().BoolVar(&debug, "debug", false, "enable detailed network request logging")
	c.Flags().BoolVarP(&verbose, "verbose", "v", false, "print each created resource")
	c.Flags().BoolVar(&recreateIDs, "recreate-ids", false, "ignore existing resource IDs and let the server generate new ones")
	return c
}

// detectAndHandleConflicts checks for version conflicts by performing a dry run on the server
func detectAndHandleConflicts(cmd *cobra.Command, ws *workspace.Workspace, _ *workspace.LockFile, _ *workspace.Meta, plan *planner.Plan, scanner *bufio.Scanner) error {
	// Perform dry run on the server
	c := client.New(ws.Config.ServerURL, ws.Config.APIKey, false)
	plan.Request.DryRun = proto.Bool(true)
	resp, err := c.ApplyWorkspacePlan(cmd.Context(), connect.NewRequest(plan.Request))
	plan.Request.DryRun = nil // reset so the real apply is not also a dry run
	if err != nil {
		return fmt.Errorf("server plan failed: %w", err)
	}

	if len(resp.Msg.Conflicts) == 0 {
		return nil
	}

	fmt.Fprintf(cmd.ErrOrStderr(), "⚠️  Version conflict detected:\n")
	if resp.Msg.Version != nil {
		fmt.Fprintf(cmd.ErrOrStderr(), "- Remote has newer version %s (%s) via %s\n",
			resp.Msg.Version.VersionId, resp.Msg.Version.CreatedAt.AsTime().Format(time.RFC3339), resp.Msg.Version.CreatedBy)
	}

	fmt.Fprintf(cmd.ErrOrStderr(), "- %d conflicts detected:\n", len(resp.Msg.Conflicts))
	for _, conflict := range resp.Msg.Conflicts {
		fmt.Fprintf(cmd.ErrOrStderr(), "  * %s \"%s\" (local %s, remote %s)\n",
			conflict.ResourceType, conflict.Ref,
			conflict.LocalUpdatedAt.AsTime().Format(time.RFC3339),
			conflict.RemoteUpdatedAt.AsTime().Format(time.RFC3339))
	}

	fmt.Fprintf(cmd.ErrOrStderr(), "\nOptions:\n")
	fmt.Fprintf(cmd.ErrOrStderr(), "[1] Abort and review changes\n")
	fmt.Fprintf(cmd.ErrOrStderr(), "[2] Force apply (overwrite remote changes)\n")
	fmt.Fprintf(cmd.ErrOrStderr(), "[3] Review conflicts one-by-one (not implemented)\n")
	fmt.Fprintf(cmd.ErrOrStderr(), "\nChoose option [1-3]: ")

	if !scanner.Scan() {
		return errors.New("no response received")
	}

	choice := strings.TrimSpace(scanner.Text())
	switch choice {
	case "1":
		fmt.Fprintln(cmd.OutOrStdout(), "Apply aborted.")
		return errors.New("apply aborted by user")
	case "2":
		fmt.Fprintln(cmd.OutOrStdout(), "Proceeding with force apply...")
		return nil
	default:
		return errors.New("invalid choice or aborted")
	}
}

func updatePlanMetadataFromResponse(wdir string, meta *workspace.Meta, ws *workspace.Workspace, plan *planner.Plan, respMsg *diagv1.ApplyPlanResponse) error {
	_ = plan
	if meta == nil {
		meta = &workspace.Meta{}
	}
	if meta.Elements == nil {
		meta.Elements = make(map[string]*workspace.ResourceMetadata)
	}
	if meta.Views == nil {
		meta.Views = make(map[string]*workspace.ResourceMetadata)
	}
	if meta.Connectors == nil {
		meta.Connectors = make(map[string]*workspace.ResourceMetadata)
	}

	for ref := range ws.Elements {
		if metadata, ok := resourceMetadataFromMap(respMsg.GetElementMetadata(), ref); ok {
			meta.Elements[ref] = metadata
		}
	}
	for ref, element := range ws.Elements {
		if !element.HasView {
			continue
		}
		if metadata, ok := resourceMetadataFromMap(respMsg.GetDiagramMetadata(), ref); ok {
			meta.Views[ref] = metadata
		}
	}
	for ref := range ws.Connectors {
		if metadata, ok := resourceMetadataFromMap(respMsg.GetConnectorMetadata(), ref); ok {
			meta.Connectors[ref] = metadata
		}
	}

	if err := workspace.WriteMetadataSection(wdir, "elements.yaml", "_meta_elements", meta.Elements); err != nil {
		return fmt.Errorf("write elements metadata: %w", err)
	}
	if err := workspace.WriteMetadataSection(wdir, "elements.yaml", "_meta_views", meta.Views); err != nil {
		return fmt.Errorf("write view metadata: %w", err)
	}
	if err := workspace.WriteMetadataSection(wdir, "connectors.yaml", "_meta_connectors", meta.Connectors); err != nil {
		return fmt.Errorf("write connector metadata: %w", err)
	}

	return nil
}

// updateLockFileFromResponse updates lock file with response data
func updateLockFileFromResponse(wdir string, existingLock *workspace.LockFile, ws *workspace.Workspace, respMsg *diagv1.ApplyPlanResponse) error {
	summary := respMsg.GetSummary()
	diagramCount := len(respMsg.GetCreatedDiagrams())
	elementCount := len(respMsg.GetCreatedElements())
	connectorCount := len(respMsg.GetCreatedConnectors())
	legacyDiagramCount := diagramCount
	legacyElementCount := elementCount
	legacyConnectorCount := connectorCount
	if summary != nil {
		legacyDiagramCount = int(summary.GetDiagramsCreated())
		legacyElementCount = int(summary.GetElementsCreated())
		legacyConnectorCount = int(summary.GetConnectorsCreated())
	}

	var lockFile *workspace.LockFile
	if existingLock != nil {
		lockFile = existingLock
	} else {
		lockFile = &workspace.LockFile{}
	}

	// Generate new version ID
	versionID := fmt.Sprintf("v%d", legacyDiagramCount+legacyElementCount+legacyConnectorCount)
	if respMsg.Version != nil {
		versionID = respMsg.Version.VersionId
	}

	// Calculate workspace hash
	workspaceHash, err := workspace.CalculateWorkspaceHash(wdir)
	if err != nil {
		return fmt.Errorf("calculate workspace hash: %w", err)
	}

	// Load current metadata from YAMLs to store in lockfile as the sync point
	meta, err := workspace.LoadMetadata(wdir)
	if err != nil {
		return fmt.Errorf("load metadata for lockfile: %w", err)
	}

	// Update lock file
	workspace.UpdateLockFile(lockFile, versionID, "cli", 0, 0, 0, 0, workspaceHash, nil, meta)
	lockFile.Resources.Elements = len(ws.Elements)
	lockFile.Resources.Views = diagramCount
	lockFile.Resources.Connectors = len(ws.Connectors)

	if err := workspace.WriteLockFile(wdir, lockFile); err != nil {
		return fmt.Errorf("write lock file: %w", err)
	}
	return nil
}

func resourceMetadataFromMap(source map[string]*diagv1.ResourceMetadata, ref string) (*workspace.ResourceMetadata, bool) {
	resourceMeta, ok := source[ref]
	if !ok || resourceMeta == nil {
		return nil, false
	}
	metadata := &workspace.ResourceMetadata{ID: workspace.ResourceID(resourceMeta.Id)}
	if resourceMeta.UpdatedAt != nil {
		metadata.UpdatedAt = resourceMeta.UpdatedAt.AsTime()
	}
	return metadata, true
}
