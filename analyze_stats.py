import yaml
import collections
import os

def analyze():
    elements_path = "../../digitaltwin-poc/elements.yaml"
    connectors_path = "../../digitaltwin-poc/connectors.yaml"

    with open(elements_path, 'r') as f:
        elements = yaml.safe_load(f)
    
    with open(connectors_path, 'r') as f:
        connectors = yaml.safe_load(f)

    # 1. Statistics
    total_elements = len(elements)
    kind_counts = collections.Counter(el.get('kind', 'unknown') for el in elements.values())
    
    # 2. Connector histogram
    outgoing = collections.defaultdict(int)
    incoming = collections.defaultdict(int)
    
    total_connectors = len(connectors)
    for conn in connectors.values():
        src = conn.get('source')
        tgt = conn.get('target')
        outgoing[src] += 1
        incoming[tgt] += 1

    def get_hist(counts, total_nodes):
        hist = collections.defaultdict(int)
        for node in elements.keys():
            hist[counts[node]] += 1
        # Also account for nodes that might be in connectors but not elements (external)
        # but the prompt asked about "how many have 0, 1, 2..." for elements.
        return hist

    out_hist = get_hist(outgoing, total_elements)
    in_hist = get_hist(incoming, total_elements)

    # 3. Duplicates (Same symbol + FilePath)
    seen_symbols = collections.defaultdict(list)
    for ref, el in elements.items():
        if 'symbol' in el and 'file_path' in el:
            key = (el['symbol'], el['file_path'])
            seen_symbols[key].append(ref)
    
    duplicates = {k: v for k, v in seen_symbols.items() if len(v) > 1}

    # 4. External lib functions / Built-ins
    # Heuristic: Elements referenced as target but not in elements.yaml
    # OR elements in elements.yaml that look like built-ins (no file_path)
    defined_refs = set(elements.keys())
    referenced_targets = set(conn.get('target') for conn in connectors.values())
    external_refs = referenced_targets - defined_refs
    
    builtin_candidates = ['str', 'set', 'int', 'dict', 'list', 'len', 'enumerate', 'zip', 'range', 'print', 'ping', 'reset']
    found_builtins = [ref for ref in defined_refs if elements[ref].get('name') in builtin_candidates]

    # 5. Boilerplate
    # Heuristic: Symbols like 'main', '__init__', 'reset', 'wrapper'
    boilerplate_names = ['main', '__init__', 'reset', 'wrapper', 'parse_args', 'init_db', 'ping']
    found_boilerplate = [ref for ref, el in elements.items() if el.get('symbol') in boilerplate_names or el.get('name') in boilerplate_names]

    # Output results
    print(f"--- GENERAL STATISTICS ---")
    print(f"Total Elements: {total_elements}")
    print(f"Total Connectors: {total_connectors}")
    print("\n--- ELEMENTS BY KIND ---")
    for kind, count in kind_counts.most_common():
        print(f"{kind}: {count}")

    print("\n--- CONNECTOR HISTOGRAM (Outgoing/Fan-out) ---")
    for count in sorted(out_hist.keys()):
        print(f"{count} outgoing: {out_hist[count]} elements")

    print("\n--- CONNECTOR HISTOGRAM (Incoming/Fan-in) ---")
    for count in sorted(in_hist.keys()):
        print(f"{count} incoming: {in_hist[count]} elements")

    print("\n--- TOP FAN-OUT (Most outgoing) ---")
    top_out = sorted(outgoing.items(), key=lambda x: x[1], reverse=True)[:10]
    for ref, count in top_out:
        name = elements.get(ref, {}).get('name', ref)
        print(f"{name} ({ref}): {count}")

    print("\n--- TOP FAN-IN (Most incoming) ---")
    top_in = sorted(incoming.items(), key=lambda x: x[1], reverse=True)[:10]
    for ref, count in top_in:
        name = elements.get(ref, {}).get('name', ref)
        print(f"{name} ({ref}): {count}")

    print("\n--- DUPLICATES (Same symbol in same file) ---")
    if not duplicates:
        print("None found")
    for (sym, path), refs in duplicates.items():
        print(f"Symbol '{sym}' in {path}: {refs}")

    print("\n--- EXTERNAL / BUILT-IN CANDIDATES ---")
    print(f"Referenced but not defined: {sorted(list(external_refs))[:20]} ... ({len(external_refs)} total)")
    print(f"Likely built-ins defined as elements: {found_builtins}")

    print("\n--- BOILERPLATE CANDIDATES ---")
    print(f"Found {len(found_boilerplate)} boilerplate-like elements (main, init, reset, etc.)")
    print(f"Examples: {found_boilerplate[:20]}")

analyze()
