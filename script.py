import json
import sys

def main():
    try:
        with open('coverage.json', 'r') as f:
            data = json.load(f)
            for file_data in data['data'][0]['files']:
                if 'src/utils/ui.rs' in file_data['filename']:
                    percent = file_data['summary']['lines']['percent']
                    print(f"Coverage for src/utils/ui.rs: {percent}%")
                    break
            else:
                print("src/utils/ui.rs not found in coverage data")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == '__main__':
    main()
