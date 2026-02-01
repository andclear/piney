import sys

def txt_to_rtf(txt_path, rtf_path):
    with open(txt_path, 'r', encoding='utf-8') as f:
        content = f.read()

    # Basic RTF header
    # \rtf1\ansi\ansicpg936 (GBK) or \ansicpg1252 (Latin1) then use unicode \uXXXX
    # \uc1 means 1 replacement char after \uXXXX
    rtf_header = r"{\rtf1\ansi\ansicpg65001\deff0\nouicompat\deflang1033{\fonttbl{\f0\fnil\fcharset134 PingFang SC;}{\f1\fnil\fcharset0 Helvetica;}}"
    # \fcharset134 is GB2312, but we use unicode escapes primarily.
    
    rtf_content = []
    rtf_content.append(rtf_header)
    rtf_content.append(r"\viewkind4\uc1\pard\sa100\sl240\slmult1\f0\fs22\lang2052")

    for char in content:
        code = ord(char)
        if code < 128:
            if char == '\n':
                rtf_content.append(r"\par ")
            elif char in ['{', '}', '\\']:
                rtf_content.append(r"\\" + char)
            else:
                rtf_content.append(char)
        else:
            # Unicode escape: \uN? where N is signed short
            # Python ord is unsigned, RTF expects signed 16-bit usually? 
            # Actually \uN takes a decimal number.
            # \uXXXX followed by a replacement character (usually ?)
            if code > 32767:
                code = code - 65536
            rtf_content.append(f"\\u{code}?")

    rtf_content.append("}")

    with open(rtf_path, 'w', encoding='ascii') as f:
        f.write("".join(rtf_content))

if __name__ == "__main__":
    txt_to_rtf("src-tauri/LICENSE.txt", "src-tauri/LICENSE.rtf")
    print("Converted to src-tauri/LICENSE.rtf")
