import yaml
import collections
import os

def analyze_folders():
    elements_path = "../../digitaltwin-poc/elements.yaml"
    connectors_path = "../../digitaltwin-poc/connectors.yaml"

    with open(elements_path, 'r') as f:
        elements = yaml.safe_load(f)
    
    with open(connectors_path, 'r') as f:
        connectors = yaml.safe_load(f)

    folder_stats = collections.defaultdict(lambda: {"total": 0, "functions": 0, "classes": 0, "files": 0})
    
    # 1. Map elements to folders
    element_to_folder = {}
    for ref, el in elements.items():
        fpath = el.get('file_path', '')
        if not fpath:
            folder = "."
        else:
            folder = os.path.dirname(fpath) or "."
        
        element_to_folder[ref] = folder
        stats = folder_stats[folder]
        stats["total"] += 1
        kind = el.get('kind', '')
        if kind == 'function': stats["functions"] += 1
        elif kind == 'class': stats["classes"] += 1
        elif kind == 'file': stats["files"] += 1

    # 2. Analyze connectors (Internal vs Cross-folder)
    folder_connectivity = collections.defaultdict(lambda: {"internal": 0, "outgoing_cross": 0, "incoming_cross": 0})
    
    for conn in connectors.values():
        src_ref = conn.get('source')
        tgt_ref = conn.get('target')
        
        src_folder = element_to_folder.get(src_ref, "unknown")
        tgt_folder = element_to_folder.get(tgt_ref, "unknown")
        
        if src_folder == tgt_folder:
            folder_connectivity[src_folder]["internal"] += 1
        else:
            folder_connectivity[src_folder]["outgoing_cross"] += 1
            folder_connectivity[tgt_folder]["incoming_cross"] += 1

    print(f"{'Folder':<40} | {'Files':<5} | {'Syms':<5} | {'Int.':<5} | {'Ext. Out':<8} | {'Ext. In':<8}")
    print("-" * 90)
    
    # Sort by total symbols
    sorted_folders = sorted(folder_stats.items(), key=lambda x: x[1]["total"], reverse=True)
    
    for folder, stats in sorted_folders:
        conn = folder_connectivity[folder]
        syms = stats["functions"] + stats["classes"]
        print(f"{folder:<40} | {stats['files']:<5} | {syms:<5} | {conn['internal']:<5} | {conn['outgoing_cross']:<8} | {conn['incoming_cross']:<8}")

analyze_folders()
