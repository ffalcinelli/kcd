import json
with open('coverage.json', 'r') as f:
    data = json.load(f)
for fd in data['data'][0]['files']:
    print(f"{fd['filename']}: {fd['summary']['lines']['percent']}%")
