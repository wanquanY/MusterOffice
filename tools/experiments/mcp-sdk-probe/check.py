"""Real stdio protocol probes, using independently framed client messages."""
import hashlib, json, os, select, subprocess, sys, time
from pathlib import Path

root=Path(sys.argv[1]); root.mkdir()
binary=Path(sys.argv[2])
expected={d['id']:d for d in json.loads(Path('.codex-work/operation-discovery/reference-1/schemas.json').read_text())}
meta={'io.modelcontextprotocol/protocolVersion':'2026-07-28',
      'io.modelcontextprotocol/clientCapabilities':{},
      'io.modelcontextprotocol/clientInfo':{'name':'independent-probe','version':'1'}}
reports=[]

def exchange(era):
    stderr=(root/f'{era}.stderr').open('xb')
    p=subprocess.Popen([binary],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=stderr)
    pending=bytearray(); calls=[]; seq=0
    def call(method,params=None,*,notification=False,metadata=None):
        nonlocal seq
        seq+=1
        params={} if params is None else params.copy()
        if era=='modern': params['_meta']=meta if metadata is None else metadata
        request=dict(jsonrpc='2.0',method=method,params=params)
        if not notification: request['id']=seq
        data=(json.dumps(request,separators=(',',':'))+'\n').encode()
        assert len(data)<=4096
        p.stdin.write(data); p.stdin.flush()
        if notification: return
        deadline=time.monotonic()+30
        while b'\n' not in pending:
            remain=deadline-time.monotonic()
            assert remain>0, 'response timeout'
            assert select.select([p.stdout],[],[],remain)[0], 'response timeout'
            part=os.read(p.stdout.fileno(),65536)
            assert part, f'server exited {p.poll()}'
            pending.extend(part)
            assert len(pending)<=8*1024*1024, 'probe response budget'
        line,rest=pending.split(b'\n',1); pending[:]=rest
        response=json.loads(line)
        (root/f'{era}-{seq:02}.request.json').write_text(json.dumps(request,indent=2)+'\n')
        (root/f'{era}-{seq:02}.response.json').write_text(json.dumps(response,indent=2)+'\n')
        assert response.get('id')==seq, response
        calls.append(dict(method=method,requestId=seq))
        return response
    try:
        if era=='legacy':
            response=call('initialize',dict(protocolVersion='2025-11-25',capabilities={},clientInfo={'name':'independent-probe','version':'1'}))
            assert response['result']['protocolVersion']=='2025-11-25'
            call('notifications/initialized',notification=True)
        else:
            response=call('server/discover')
            assert 'result' in response,response
        tools=call('tools/list')['result']['tools']; assert len(tools)==1
        assert tools[0]['name']=='mo_probe_schema'
        assert tools[0]['outputSchema']==json.loads(Path('contracts/generated/schema-document.schema.json').read_text())
        for identifier,document in expected.items():
            result=call('tools/call',dict(name='mo_probe_schema',arguments={'id':identifier}))['result']
            assert result['structuredContent']==document
            assert json.loads(result['content'][0]['text'])==document
            assert not result['isError']
            if era=='legacy': assert 'resultType' not in result
            else: assert result['resultType']=='complete'
        for arguments in [{'id':'not-a-schema'},{'id':'host-request','principal':'forged'}]:
            error=call('tools/call',dict(name='mo_probe_schema',arguments=arguments))['error']
            assert error['code']==-32602,error
        error=call('tools/call',dict(name='missing',arguments={}))['error']
        assert error['code']==-32601,error
        metadata_rejections=0
        if era=='modern':
            error=call('tools/list',metadata=meta|{'io.modelcontextprotocol/protocolVersion':'2099-01-01'})['error']
            assert error['code']==-32022,error
            missing={k:v for k,v in meta.items() if k!='io.modelcontextprotocol/clientCapabilities'}
            error=call('tools/list',metadata=missing)['error']
            # MCP 2026-07-28 basic/_meta explicitly requires Invalid params.
            assert error['code']==-32602,error
            metadata_rejections=2
        p.stdin.close(); assert p.wait(timeout=15)==0
        assert not pending
        reports.append(dict(era=era,calls=calls,schemaPairs=len(expected),rejectedQueries=3,rejectedMetadata=metadata_rejections))
    finally:
        if p.poll() is None: p.kill(); p.wait()
        stderr.close()

exchange('legacy'); exchange('modern')
oversize=subprocess.run([binary],input=b'x'*4097+b'\n',capture_output=True,timeout=15)
(root/'over-budget.stdout').write_bytes(oversize.stdout)
(root/'over-budget.stderr').write_bytes(oversize.stderr)
assert oversize.returncode!=0 and not oversize.stdout
report=dict(binary=dict(path=str(binary),sha256=hashlib.sha256(binary.read_bytes()).hexdigest()),
    sessions=reports,inputByteLimit=4096,overBudgetExit=oversize.returncode,
    scope='SDK selection probe against real shared schema computation; not production MCP tools, scheduler, resource transport or product acceptance.')
(root/'report.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(schemaPairs=20,protocols=['2025-11-25','2026-07-28'],rejectedQueries=6,rejectedMetadata=2,overBudgetRejected=True)))
