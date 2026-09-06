"""Compile the unmodified upstream rate functions to generate differential fixtures."""
import hashlib,json,pathlib,re,subprocess,tempfile,urllib.request
ROOT=pathlib.Path(__file__).resolve().parents[1]
results=[]
for version in ['4.3.0','4.4.0','4.5.0']:
    url=f'https://raw.githubusercontent.com/betaflight/betaflight/{version}/src/main/fc/rc.c'
    data=urllib.request.urlopen(url).read(); source=data.decode()
    functions=[]
    for name in ['Betaflight','Kiss','Actual','Quick']:
        start=source.index(f'float apply{name}Rates('); end=source.index('\n}',start)+2
        functions.append(source[start:end])
    shim='''#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <math.h>
#define power3(x) ((x)*(x)*(x))
#define power5(x) ((x)*(x)*(x)*(x)*(x))
#define MAX(a,b) ((a)>(b)?(a):(b))
#define constrainf(x,a,b) fminf(fmaxf(x,a),b)
#define RC_RATE_INCREMENTAL 14.54f
#define SETPOINT_RATE_LIMIT 1998.0f
#define SETPOINT_RATE_LIMIT_MIN -1998.0f
#define SETPOINT_RATE_LIMIT_MAX 1998.0f
struct Profile {uint8_t rcExpo[3]; uint8_t rcRates[3]; uint8_t rates[3]; bool quickRatesRcExpo;};
struct Profile profile;
struct Profile *currentControlRateProfile=&profile;
'''
    main='''
int main(void) {
float (*models[])(int,float,float)={applyBetaflightRates,applyKissRates,applyActualRates,applyQuickRates};
const char *names[]={"BETAFLIGHT","KISS","ACTUAL","QUICK"};
int cases[][3]={{20,80,50},{100,70,0},{230,90,100},{80,20,35}};
for(int m=0;m<4;m++)for(int c=0;c<4;c++)for(int q=0;q<2;q++)for(int i=-10;i<=10;i++) {
profile.rcRates[0]=cases[c][0];profile.rates[0]=cases[c][1];profile.rcExpo[0]=cases[c][2];profile.quickRatesRcExpo=q;
float x=(float)i/10;
printf("%s,%d,%d,%d,%d,%.9g,%.9g\\n",names[m],cases[c][0],cases[c][1],cases[c][2],q,x,models[m](0,x,fabsf(x)));
}}
'''
    with tempfile.TemporaryDirectory() as tmp:
        path=pathlib.Path(tmp);(path/'reference.c').write_text(shim+'\n'.join(functions)+main)
        subprocess.run(['cc','-O0',str(path/'reference.c'),'-lm','-o',str(path/'reference')],check=True)
        output=subprocess.check_output([str(path/'reference')],text=True)
    for line in output.splitlines():
        model,rc,sr,e,q,x,y=line.split(',')
        results.append({'version':version,'model':model,'rc':float(rc),'superRate':float(sr),'expo':float(e),'quickExpo':q=='1','x':float(x),'y':float(y)})
    print(version,hashlib.sha256(data).hexdigest())
(ROOT/'fixtures/rate-vectors.json').write_text(json.dumps(results,separators=(',',':'))+'\n')
