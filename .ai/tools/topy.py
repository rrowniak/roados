import re,html,sys,glob,os
def txt(p):
    s=open(p,encoding='utf-8',errors='replace').read()
    s=re.sub(r'<(script|style|nav|footer)[^>]*>.*?</\1>','',s,flags=re.S|re.I)
    s=re.sub(r'<br\s*/?>','\n',s,flags=re.I)
    s=re.sub(r'</(p|div|li|tr|h[1-6]|td|th)>','\n',s,flags=re.I)
    s=re.sub(r'<[^>]+>','',s)
    s=html.unescape(s)
    s=re.sub(r'[ \t\xa0]+',' ',s)
    return re.sub(r'\n\s*\n+','\n',s).strip()
for p in sys.argv[1:]:
    for f in (sorted(glob.glob(p)) if '*' in p else [p]):
        print('='*20,f)
        print(txt(f))
