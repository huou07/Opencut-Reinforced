#!/usr/bin/env python3
"""Validate routes and report documentation context guidance without writing files."""
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from scripts.document_routing import RoutingError, load, validate


def main():
    try:
        root = Path(__file__).resolve().parents[1]
        errors, warnings, bundles = validate(root, load(root))
        print(json.dumps({'valid': not errors, 'errors': errors, 'budget_warnings': warnings, 'bundles': bundles}, indent=2))
        return bool(errors)
    except (RoutingError, OSError, ValueError) as exc:
        print(f'FAIL: {exc}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    raise SystemExit(main())
