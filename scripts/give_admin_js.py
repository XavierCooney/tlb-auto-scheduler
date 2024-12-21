#!/usr/bin/env python3

# Usage: scripts/give_admin_js.py output/latest/solution.tsv

import sys, csv, pathlib, json

with open(sys.argv[1]) as solution:
    print("document.getElementsByName('changed')[0].value = 1;", end=' ')
    for assignment in csv.DictReader(solution, delimiter='\t'):
        num = {'tut+lab': 0, 'lab': 1}[assignment['type']]
        class_name = assignment['class']
        tutor_zid = json.dumps(assignment['zid'])
        print(f"document.getElementsByName('tutor_{class_name}_{num}')[0].value = {tutor_zid};", end=' ')
print()
