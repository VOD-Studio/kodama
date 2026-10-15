import math
try:
    from PIL import Image
except ImportError:  # only needed for the debug sheet
    Image = None
W,H=49,48
PAL={'O':'#2f2362','P':'#5b4a9c','a':'#ffffff','b':'#efebfa','c':'#d5ccf0','d':'#aea0de','e':'#8779c5',
 'H':'#f6f2fe','J':'#ddd4f3','K':'#b7aadd','L':'#8f80c8','G':'#f2c14e','B':'#9486c8',
 's':'#ffe9e0','t':'#f6c8bc','k':'#f7b0ba','m':'#d9707e','M':'#a8485a',
 'i':'#5f4cc4','I':'#8d7ae6','F':'#c9bdf8','w':'#ffffff',
 'W':'#ffffff','X':'#eae6f7','Y':'#cfc8e8','Z':'#aaa0d4','g':'#b8c0d6','n':'#8c96b4',
 'V':'#e4d9f6','U':'#b9acdf','T':'#8f80c8','S':'#f3effa','R':'#d6cbea','Q':'#b0a3d3','N':'#8a7dc0',
 '1':'#34c77a','2':'#1c8f52','3':'#ea4b62','4':'#a42a42','5':'#5aa0ff','6':'#ff82a2','7':'#8e97b0','8':'#4a526c','9':'#b07a4a','0':'#d8a56f','y':'#fff2a8'}
class Img:
    def __init__(s,w=W,h=H): s.w,s.h=w,h; s.g=[['.']*w for _ in range(h)]
    def put(s,x,y,c):
        if 0<=x<s.w and 0<=y<s.h and c!=' ': s.g[y][x]=c
    def P(s,y,x,t):
        for i,c in enumerate(t): s.put(x+i,y,c) if c!='.' else None
    def span(s,y,x0,x1,c):
        for x in range(x0,x1+1): s.put(x,y,c)
    def get(s,x,y): return s.g[y][x] if 0<=x<s.w and 0<=y<s.h else '.'
    def stroke(s,pts,rad,fill,outline='O',tone=None):
        """Brush along a polyline with varying radius; returns the painted cell set and outlines it."""
        cells=set()
        n=len(pts)
        path=[]
        for i in range(n-1):
            (x0,y0),(x1,y1)=pts[i],pts[i+1]
            steps=max(1,int(max(abs(x1-x0),abs(y1-y0))*3))
            for k in range(steps+1):
                f=k/steps
                path.append((x0+(x1-x0)*f,y0+(y1-y0)*f,rad[i]+(rad[i+1]-rad[i])*f))
        for (px,py,r) in path:
            for yy in range(int(py-r-1),int(py+r+2)):
                for xx in range(int(px-r-1),int(px+r+2)):
                    if (xx-px)**2+(yy-py)**2<=r*r+0.25: cells.add((xx,yy))
        for (x,y) in cells: s.put(x,y,tone(x,y) if tone else fill)
        if outline:
            for (x,y) in list(cells):
                for dx,dy in ((1,0),(-1,0),(0,1),(0,-1)):
                    if (x+dx,y+dy) not in cells and s.get(x+dx,y+dy)=='.': s.put(x+dx,y+dy,outline)
        return cells
    def text(s): return '\n'.join(''.join(r) for r in s.g)
    def png(s,path,scale=10,bg=(236,233,226),pal=PAL):
        im=Image.new('RGB',(s.w,s.h),bg)
        for y in range(s.h):
            for x in range(s.w):
                c=s.g[y][x]
                if c!='.': v=pal[c]; im.putpixel((x,y),(int(v[1:3],16),int(v[3:5],16),int(v[5:7],16)))
        im.resize((s.w*scale,s.h*scale),Image.NEAREST).save(path)
