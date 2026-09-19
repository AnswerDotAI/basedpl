"""Scan, review and activate reference cases without regenerating the corpus."""
import argparse
from miniapl.reference import scan, review, activate


def main():
    parser = argparse.ArgumentParser(description='Scan, review and activate reference cases without regenerating the corpus.')
    parser.add_argument('action', choices=['scan', 'show', 'activate'])
    parser.add_argument('--report', default='meta/reference-scan.json')
    parser.add_argument('--source', default='')
    parser.add_argument('--match', default='')
    parser.add_argument('--timeout', type=float, default=.25)
    parser.add_argument('--status', default='pass', help='show: empty string selects every outcome')
    parser.add_argument('--limit', type=int, default=20, help='show: 0 displays all matches')
    parser.add_argument('--details', action='store_true')
    args = parser.parse_args()
    common = dict(report=args.report, source=args.source, match=args.match)
    if args.action == 'scan': scan(timeout=args.timeout, **common)
    elif args.action == 'show': review(status=args.status, limit=args.limit, details=args.details, **common)
    else: activate(**common)


if __name__ == '__main__': main()
