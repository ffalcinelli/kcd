import json
import sys

def main():
    try:
        with open('coverage.json', 'r') as f:
            data = json.load(f)
            for file_data in data['data'][0]['files']:
                if 'src/utils/ui.rs' in file_data['filename']:
                    for line in file_data['lines']:
                        if line['count'] == 0:
                            print(f"Line {line['line_number']} is uncovered")
                    break
            else:
                print("src/utils/ui.rs not found in coverage data")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == '__main__':
    main()
