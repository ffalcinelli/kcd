import json
with open('coverage.json', 'r') as f:
    data = json.load(f)
for fd in data['data'][0]['files']:
    if 'ui.rs' in fd['filename']:
        print(fd['summary']['lines']['covered'])
        print(fd['summary']['lines']['count'])
