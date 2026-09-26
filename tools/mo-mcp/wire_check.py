"""Independent Unix subprocess wire client shared by fault/lifecycle checks."""
import json
import os
import select
import subprocess
import time

META = {'io.modelcontextprotocol/protocolVersion': '2026-07-28',
        'io.modelcontextprotocol/clientCapabilities': {},
        'io.modelcontextprotocol/clientInfo': {'name': 'transport-verifier', 'version': '1'}}


class Client:
    def __init__(self, root, binary, config, name, era='modern', expected_exit=0):
        self.root = root; self.name = name; self.era = era; self.expected_exit = expected_exit
        self.pending = bytearray(); self.seq = 100; self.frames = []
        self.log = (root/f'{name}.stderr').open('xb')
        self.input = (root/f'{name}.input.bin').open('xb')
        self.output = (root/f'{name}.output.bin').open('xb')
        self.p = subprocess.Popen([binary, config], stdin=subprocess.PIPE,
            stdout=subprocess.PIPE, stderr=self.log)

    def __enter__(self): return self

    def __exit__(self, kind, *_):
        try:
            self.p.stdin.close()
            if kind and self.p.poll() is None: self.p.kill()
            code = self.p.wait(timeout=20)
            if not kind: assert code == self.expected_exit, (self.name, code)
        finally:
            if self.p.poll() is None: self.p.kill(); self.p.wait()
            self.p.stdout.close(); self.log.close(); self.input.close(); self.output.close()
            (self.root/f'{self.name}.frames.json').write_text(json.dumps(self.frames, indent=2)+'\n')

    def raw(self, data):
        self.input.write(data); self.input.flush()
        self.p.stdin.write(data); self.p.stdin.flush()

    def send(self, method, params=None, *, notification=False):
        self.seq += 1
        params = dict(params or {})
        if self.era == 'modern': params['_meta'] = META
        value = dict(jsonrpc='2.0', method=method, params=params)
        if not notification: value['id'] = self.seq
        self.raw((json.dumps(value, separators=(',', ':'))+'\n').encode())
        return self.seq

    def receive(self):
        deadline = time.monotonic()+15
        while b'\n' not in self.pending:
            remaining = deadline-time.monotonic()
            assert remaining > 0 and select.select([self.p.stdout], [], [], remaining)[0], self.name
            part = os.read(self.p.stdout.fileno(), 65536)
            assert part, f'{self.name}: unexpected EOF'
            self.pending.extend(part)
            assert len(self.pending) <= 16*1024*1024+1
        line, rest = self.pending.split(b'\n', 1); self.pending[:] = rest
        self.output.write(line+b'\n'); self.output.flush()
        value = json.loads(line); self.frames.append(value)
        return value

    def call(self, method, params=None):
        identity = self.send(method, params)
        value = self.receive(); assert value['id'] == identity, value
        return value

    def start(self):
        if self.era == 'legacy':
            value = self.call('initialize', dict(protocolVersion='2025-11-25', capabilities={},
                clientInfo={'name': 'transport-verifier', 'version': '1'}))
            assert value['result']['protocolVersion'] == '2025-11-25'
            self.send('notifications/initialized', notification=True)
        else:
            assert 'result' in self.call('server/discover')
