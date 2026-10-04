"""Strict, read-only documentation selection; no execution or authority grants."""
from __future__ import annotations

import json
import re
from pathlib import Path, PurePosixPath
from urllib.parse import unquote, urlsplit

MANIFEST = 'docs/DOC_ROUTING.json'
KEY = re.compile(r'^[a-z][a-z0-9-]*$')


class RoutingError(ValueError):
    pass


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise RoutingError(f'duplicate JSON key: {key}')
        result[key] = value
    return result


def _path(root, value):
    if not isinstance(value, str) or not value or '\\' in value:
        raise RoutingError(f'invalid documentation path: {value!r}')
    path = PurePosixPath(value)
    if path.is_absolute() or ':' in value or any(p in ('', '.', '..') for p in value.split('/')):
        raise RoutingError(f'unsafe documentation path: {value}')
    target = root / value
    if not target.is_file() or not target.resolve().is_relative_to(root.resolve()):
        raise RoutingError(f'missing or escaping documentation target: {value}')
    if root.is_symlink() or any((root / Path(*path.parts[:i])).is_symlink() for i in range(1, len(path.parts) + 1)):
        raise RoutingError(f'symlink documentation target: {value}')
    return target


def _headings(lines):
    fenced = False
    fence = ''
    for index, line in enumerate(lines):
        stripped = line.lstrip()
        if stripped.startswith(('```', '~~~')):
            marker = stripped[:3]
            if not fenced:
                fenced, fence = True, marker
            elif marker == fence:
                fenced = False
            continue
        match = re.match(r'^(#{1,6}) (.+?)\s*\n?$', line)
        if match and not fenced:
            yield index, len(match[1]), match[2]


def section(root, entry):
    lines = _path(root, entry['path']).read_text(encoding='utf-8').splitlines(keepends=True)
    if 'heading' not in entry:
        return lines, (0, len(lines))
    headings = list(_headings(lines))
    matches = [(i, level) for i, level, title in headings if title == entry['heading']]
    if len(matches) != 1:
        raise RoutingError(f"heading must occur exactly once: {entry}")
    start, level = matches[0]
    end = next((i for i, depth, _ in headings if i > start and depth <= level), len(lines))
    return lines, (start, end)


def _refs(values, documents, label):
    if not isinstance(values, list) or not values or any(not isinstance(v, str) for v in values):
        raise RoutingError(f'{label} must be a nonempty list of document IDs')
    if len(values) != len(set(values)) or any(v not in documents for v in values):
        raise RoutingError(f'duplicate or unknown document ID in {label}')


def load(root):
    try:
        manifest = json.loads(_path(root, MANIFEST).read_text(encoding='utf-8'), object_pairs_hook=_pairs,
                              parse_constant=lambda v: (_ for _ in ()).throw(RoutingError(f'invalid number: {v}')))
    except (OSError, json.JSONDecodeError) as exc:
        raise RoutingError(str(exc)) from exc
    fields = {'schema_version', 'core', 'documents', 'features', 'phase_features', 'checkpoint_features', 'on_demand', 'budgets'}
    if not isinstance(manifest, dict) or set(manifest) != fields or type(manifest['schema_version']) is not int or manifest['schema_version'] != 1:
        raise RoutingError('unsupported or invalid routing schema')
    documents, features = manifest['documents'], manifest['features']
    if not isinstance(documents, dict) or not documents or not isinstance(features, dict) or not features:
        raise RoutingError('documents/features must be nonempty objects')
    targets = set()
    for key, entry in documents.items():
        if not KEY.fullmatch(key) or not isinstance(entry, dict) or set(entry) not in ({'path'}, {'path', 'heading'}):
            raise RoutingError(f'invalid document entry: {key}')
        if 'heading' in entry and (not isinstance(entry['heading'], str) or not entry['heading']):
            raise RoutingError(f'invalid heading: {key}')
        section(root, entry)
        target = (entry['path'], entry.get('heading'))
        if target in targets:
            raise RoutingError(f'duplicate authority selector: {target}')
        targets.add(target)
    _refs(manifest['core'], documents, 'core')
    used = set(manifest['core'])
    for key, refs in features.items():
        if not KEY.fullmatch(key) or key == 'core':
            raise RoutingError(f'invalid feature key: {key}')
        _refs(refs, documents, key)
        used.update(refs)
    if used != set(documents):
        raise RoutingError(f'unused document selectors: {sorted(set(documents) - used)}')
    for field in ('phase_features', 'checkpoint_features'):
        if not isinstance(manifest[field], dict):
            raise RoutingError(f'{field} must be an object')
        for key, refs in manifest[field].items():
            pattern = r'[0-9]+' if field == 'phase_features' else r'[0-9]+[A-Z][A-Z0-9-]*'
            if not re.fullmatch(pattern, key):
                raise RoutingError(f'invalid {field} key: {key}')
            _refs(refs, features, field)
    budgets = manifest['budgets']
    if not isinstance(budgets, dict) or set(budgets) != {'core', 'feature', 'context'} or any(type(v) is not int or v <= 0 for v in budgets.values()):
        raise RoutingError('invalid budgets')
    demand = manifest['on_demand']
    if not isinstance(demand, list) or any(not isinstance(v, str) for v in demand) or len(demand) != len(set(demand)):
        raise RoutingError('invalid or duplicate on_demand paths')
    for path in demand:
        _path(root, path)
    return manifest



def checkpoint_ranges(root, checkpoint):
    """Retain the complete chosen contract and all shared phase text."""
    try:
        from .execution_plan import load_plan_state
    except ImportError:
        from execution_plan import load_plan_state
    plan, _ = load_plan_state(root)
    peers = {c['id'] for c in plan['checkpoints'] if c['spec_document'] == checkpoint['spec_document']}
    path = checkpoint['spec_document']
    lines, _ = section(root, {'path': path})
    headings = list(_headings(lines))
    owned = [(i, level, title, title.split(' — ', 1)[0]) for i, level, title in headings
             if title.split(' — ', 1)[0] in peers and ' — ' in title]
    if sum(owner == checkpoint['id'] for _, _, _, owner in owned) != 1:
        raise RoutingError(f"checkpoint heading missing or ambiguous: {checkpoint['id']}")
    excluded = []
    for start, level, _, owner in owned:
        if owner != checkpoint['id']:
            end = next((i for i, depth, _ in headings if i > start and depth <= level), len(lines))
            excluded.append((start, end))
    ranges, cursor = [], 0
    for start, end in sorted(excluded):
        if cursor < start:
            ranges.append((cursor, start))
        cursor = max(cursor, end)
    if cursor < len(lines):
        ranges.append((cursor, len(lines)))
    return lines, ranges


def resolve(root, manifest, features=(), checkpoint=None):
    features = sorted(set(features))
    if not features and checkpoint is not None:
        features = manifest['checkpoint_features'].get(checkpoint['id'], manifest['phase_features'].get(str(checkpoint['phase']), []))
    if any(key not in manifest['features'] for key in features):
        raise RoutingError(f'unknown feature bundle: {features}')
    ids = list(manifest['core'])
    for feature in features:
        ids.extend(manifest['features'][feature])
    entries = [manifest['documents'][key] for key in dict.fromkeys(ids)]
    grouped = {}
    for entry in entries:
        lines, (start, end) = section(root, entry)
        group = grouped.setdefault(entry['path'], {'lines': lines, 'ranges': []})
        group['ranges'].append((start, end))
    if checkpoint is not None:
        lines, ranges = checkpoint_ranges(root, checkpoint)
        grouped[checkpoint['spec_document']] = {'lines': lines, 'ranges': ranges}
    files = []
    for path, group in grouped.items():
        merged = []
        for start, end in sorted(group['ranges']):
            if merged and start <= merged[-1][1]:
                merged[-1] = (merged[-1][0], max(end, merged[-1][1]))
            else:
                merged.append((start, end))
        excerpts = [{'start_line': start + 1, 'end_line': end, 'text': ''.join(group['lines'][start:end])} for start, end in merged]
        files.append({'path': path, 'bytes': sum(len(e['text'].encode('utf-8')) for e in excerpts), 'excerpts': excerpts})
    total = sum(f['bytes'] for f in files)
    return {'features': features, 'files': files, 'file_count': len(files), 'bytes': total,
            'budget_bytes': manifest['budgets']['context'], 'over_budget': total > manifest['budgets']['context']}


def summary(bundle):
    return {**bundle, 'files': [{**f, 'excerpts': [{k: v for k, v in e.items() if k != 'text'} for e in f['excerpts']]} for f in bundle['files']]}


def check_links(root):
    """Check local inline Markdown destinations; external URLs/anchors are not fetched."""
    failures = []
    paths = sorted(root.glob('*.md')) + sorted((root / 'docs').rglob('*.md'))
    for path in paths:
        for match in re.finditer(r'\[[^\]\n]*\]\((<[^>]+>|[^\s)]+)(?:\s+"[^"]*")?\)', path.read_text(encoding='utf-8')):
            raw = match[1].strip('<>')
            url = urlsplit(raw)
            if url.scheme or url.netloc or not url.path:
                continue
            target = (path.parent / unquote(url.path)).resolve()
            if not target.is_relative_to(root.resolve()) or not target.exists():
                failures.append(f'{path.relative_to(root)} -> {raw}')
    return failures


# Policy is structural; patterns cover observed regressions, not arbitrary English.
ENTRYPOINTS = ('README.md', 'AGENTS.md', 'docs/INDEX.md', 'docs/DOCUMENTATION.md',
               'docs/DEVELOPMENT_WORKFLOW.md', 'docs/PRODUCT.md')
README_HEADINGS = ('Execution and implementation evidence', 'Vision', 'Architecture direction',
                   'Product direction', 'Platforms', 'Design and documentation', 'Contributing', 'License')
STALE_CLAIMS = re.compile(r'(?:export|viewer)(?: support)? (?:is |are )?not implemented|'
                         r'Android SAF is unavailable|current (?:project|\.orproj) schema (?:is )?(?:v|version )?\d+', re.I)


def narrative_errors(root):
    errors = []
    for name in ENTRYPOINTS:
        path = root / name
        if not path.exists():
            continue  # Small resolver fixtures need not instantiate the whole repo.
        text = path.read_text()
        if STALE_CLAIMS.search(text):
            errors.append('independent mutable implementation claim: ' + name)
        if name == 'README.md':
            titles = tuple(title for _, level, title in _headings(text.splitlines()) if level == 2)
            if titles != README_HEADINGS:
                errors.append('README must retain timeless entrypoint outline; no implementation inventory')
    for name in ('ARCHITECTURE', 'TECHNICAL_PLAN', 'TESTING', 'TOOLING'):
        path = root / ('docs/' + name + '.md')
        if not path.exists():
            continue
        lines = path.read_text().splitlines(keepends=True)
        for index, level, title in _headings(lines):
            if level == 2 and title != 'Status' and not any('Historical implementation/coverage notes retained from Git baseline' in line for line in lines[index+1:index+4]):
                errors.append('missing historical section provenance: ' + str(path.relative_to(root)) + ': ' + title)
    return errors


def validate(root, manifest):
    errors = check_links(root) + narrative_errors(root)
    routed = {entry['path'] for entry in manifest['documents'].values()} | set(manifest['on_demand'])
    for path in (root / 'docs/features').rglob('*.md'):
        if str(path.relative_to(root)) not in routed:
            errors.append(f'orphan feature document: {path.relative_to(root)}')
    # Exact mutable status declarations, not historical evidence or examples.
    active = routed - set(manifest['on_demand'])
    for name in sorted(active):
        if name.endswith('.md'):
            for line in (root / name).read_text().splitlines():
                if re.match(r'^\s*(?:[-*]\s+)?(?:\*\*)?(?:Status:|[0-9]+[A-Z][A-Z0-9-]*\s*[=:])\s*(?:NEXT|PLANNED|DONE|IN_PROGRESS)\b', line):
                    errors.append(f'duplicated mutable execution status: {name}: {line}')
    try:
        from .execution_plan import load_plan_state
    except ImportError:
        from execution_plan import load_plan_state
    plan, _ = load_plan_state(root)
    checkpoints = {c['id']: c for c in plan['checkpoints']}
    if set(manifest['checkpoint_features']) - checkpoints.keys():
        errors.append('routing names unknown checkpoint IDs')
    if set(manifest['phase_features']) - {str(c['phase']) for c in checkpoints.values()}:
        errors.append('routing names unknown phases')
    reports = {'core': resolve(root, {**manifest, 'core': manifest['core']})}
    for key in manifest['features']:
        reports[key] = resolve(root, {**manifest, 'core': []}, [key])
    warnings = []
    for key, bundle in reports.items():
        limit = manifest['budgets']['core' if key == 'core' else 'feature']
        if bundle['bytes'] > limit:
            warnings.append({'bundle': key, 'bytes': bundle['bytes'], 'guidance_bytes': limit,
                             'dominant_files': sorted([(f['path'], f['bytes']) for f in bundle['files']], key=lambda v: -v[1])[:3]})
    for checkpoint in checkpoints.values():
        bundle = resolve(root, manifest, checkpoint=checkpoint)
        if bundle['over_budget']:
            warnings.append({'context': checkpoint['id'], 'bytes': bundle['bytes'],
                             'guidance_bytes': manifest['budgets']['context'],
                             'dominant_files': sorted([(f['path'], f['bytes']) for f in bundle['files']], key=lambda v: -v[1])[:3]})
    return errors, warnings, {key: {'bytes': b['bytes'], 'file_count': b['file_count']} for key, b in reports.items()}
