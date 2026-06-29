import json
with open('coverage.json', 'r') as f:
    data = json.load(f)
for fd in data['data'][0]['files']:
    if 'ui.rs' in fd['filename']:
        lines = fd['lines']
        for l in lines:
            if l['count'] == 0:
                print(f"Line {l['line_number']} is uncovered")
