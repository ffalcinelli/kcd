import sys

def main():
    try:
        with open('coverage.txt', 'r') as f:
            lines = f.readlines()

            in_file = False
            for i, line in enumerate(lines):
                if line.startswith('/app/src/utils/ui.rs:'):
                    in_file = True
                    continue
                if line.startswith('/app/'):
                    if in_file: break
                    in_file = False

                if in_file:
                    # check if column contains "0|"
                    parts = line.split('|')
                    if len(parts) >= 2:
                        count = parts[1].strip()
                        if count == "0":
                            print(f"Line: {line.strip()}")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == '__main__':
    main()
