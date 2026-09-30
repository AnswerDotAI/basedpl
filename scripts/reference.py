'Scan, review, activate and add reference cases without regenerating the corpus.'
import argparse, json
from basedpl.bpltests import add
from basedpl.reference import scan, review, activate


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('action', choices=['scan', 'show', 'activate', 'add'])
    parser.add_argument('ids', nargs='*', help='add: inventory IDs to append to the reference files')
    parser.add_argument('--report', default='meta/reference-scan.json')
    parser.add_argument('--source', default='')
    parser.add_argument('--match', default='')
    parser.add_argument('--timeout', type=float, default=.25)
    parser.add_argument('--status', default='pass', help='show: empty string selects every outcome')
    parser.add_argument('--limit', type=int, default=20, help='show: 0 displays all matches')
    parser.add_argument('--details', action='store_true')
    parser.add_argument('--directory', default='tests/reference/inventory', help='add: inventory directory')
    parser.add_argument('--output', default='tests/reference', help='add: directory of the .bpl files')
    args = parser.parse_args()
    common = dict(report=args.report, source=args.source, match=args.match)
    if args.action == 'add': print(json.dumps(add(args.ids, args.output, args.directory), ensure_ascii=False, indent=2))
    elif args.action == 'scan': scan(timeout=args.timeout, **common)
    elif args.action == 'show': review(status=args.status, limit=args.limit, details=args.details, **common)
    else: activate(**common)


if __name__ == '__main__': main()
