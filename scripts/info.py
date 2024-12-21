#!/usr/bin/env python3

import sys, json, pathlib, csv

config = pathlib.Path(sys.argv[1])

with open(config / 'talloc_cache.json') as talloc_file:
    talloc = json.load(talloc_file)

with open(config / 'instructors.tsv') as instructors_file:
    for instructor in csv.DictReader(instructors_file, delimiter='\t'):
        assert instructor['ignore'] in ('', '1')
        if instructor['ignore'] == '1':
            continue

        zid = instructor['zid']
        matching = [app for app in talloc if app['profile']['zid'] == zid]
        assert len(matching) == 1, zid
        app = matching[0]
        application = app['application']
        for field, expected in [
            ('courseReason', ''),
            ('country', 'Australia'),
            ('pref_1', 'COMP1521'),
            ('otherInfo', ''),
        ]:
            response = application.get(field, None)
            if response != expected:
                print(zid, instructor['name'], field, repr(response))
