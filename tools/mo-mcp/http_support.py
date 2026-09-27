"""Independent HTTP/SSE client: no rmcp client or server implementation imports."""
import http.client
import json
from pathlib import Path
import select
import signal
import socket
import subprocess
from urllib.parse import urlsplit

from compute_support import Session, write

VERSION = '2026-07-28'


class Server:
    def __init__(self, binary, files, name='http'):
        self.files = files
        self.directory = files.root / name
        self.directory.mkdir()
        self.stderr = self.directory / 'stderr.log'
        self.log = self.stderr.open('xb')
        self.p = subprocess.Popen([str(Path(binary).resolve()), str(files.config), '127.0.0.1:0'],
                                  stdout=subprocess.PIPE, stderr=self.log)

    def __enter__(self):
        try:
            assert select.select([self.p.stdout], [], [], 15)[0], 'listener startup timed out'
            self.info = json.loads(self.p.stdout.readline())
            assert self.info['protocol'] == VERSION
            self.url = urlsplit(self.info['url'])
            return self
        except BaseException:
            self.__exit__(True)
            raise

    def stop(self):
        if self.p.poll() is None:
            self.p.send_signal(signal.SIGINT)
        return self.p.wait(timeout=15)

    def __exit__(self, kind, *_):
        try:
            code = self.stop()
        finally:
            if self.p.poll() is None:
                self.p.kill(); self.p.wait()
            self.p.stdout.close(); self.log.close()
        if not kind:
            assert code == 0 and not self.stderr.read_bytes(), self.stderr.read_text()

    def headers(self, method, params):
        value = {'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream',
                 'MCP-Protocol-Version': VERSION, 'Mcp-Method': method}
        if method == 'tools/call':
            value['Mcp-Name'] = params['name']
        elif method == 'resources/read':
            value['Mcp-Name'] = params['uri']
        return value

    def request(self, message, override=None, verb='POST', raw=None):
        headers = self.headers(message['method'], message.get('params', {}))
        for name, value in (override or {}).items():
            if value is None:
                headers.pop(name, None)
            else:
                headers[name] = value
        data = json.dumps(message).encode() if raw is None else raw
        connection = http.client.HTTPConnection(self.url.hostname, self.url.port, timeout=20)
        try:
            connection.request(verb, self.url.path, body=data, headers=headers)
            response = connection.getresponse()
            body = response.read(16 * 1024 * 1024 + 1)
            assert len(body) <= 16 * 1024 * 1024
            return response.status, dict(response.getheaders()), body
        finally:
            connection.close()

    def pending(self, message, partial=False, extra_headers=()):
        """Return a caller-owned socket, including requests with an incomplete body."""
        data = json.dumps(message).encode()
        headers = [('Host', self.url.netloc), *self.headers(message['method'], message.get('params', {})).items(),
                   ('Content-Length', str(len(data))), *extra_headers]
        raw = ('POST ' + self.url.path + ' HTTP/1.1\r\n' +
               ''.join(f'{k}: {v}\r\n' for k, v in headers) + '\r\n').encode()
        sock = socket.create_connection((self.url.hostname, self.url.port), timeout=15)
        try:
            sock.sendall(raw + (data[:1] if partial else data))
        except BaseException:
            sock.close()
            raise
        return sock


def rpc(method, params=None, request_id=1):
    params = dict(params or {})
    params['_meta'] = {'io.modelcontextprotocol/protocolVersion': VERSION,
                       'io.modelcontextprotocol/clientCapabilities': {},
                       'io.modelcontextprotocol/clientInfo': {'name': 'independent-http-check', 'version': '1'}}
    return dict(jsonrpc='2.0', id=request_id, method=method, params=params)


def decode(headers, data):
    if headers.get('content-type', '').startswith('text/event-stream'):
        messages = []
        for event in data.replace(b'\r\n', b'\n').split(b'\n\n'):
            fields = event.split(b'\n')
            assert not any(f.startswith((b'id:', b'retry:')) for f in fields), 'unexpected resumable SSE'
            payload = b'\n'.join(f[5:].lstrip(b' ') for f in fields if f.startswith(b'data:'))
            if payload:
                messages.append(json.loads(payload))
        responses = [m for m in messages if 'id' in m]
        assert len(responses) == 1, messages
        return responses[0]
    return json.loads(data)


class Client(Session):
    # Reuse the independent shared schema/result/asset checks, not its transport.
    def __init__(self, server, name='protocol'):
        self.server, self.files, self.era, self.seq = server, server.files, VERSION, 0
        self.directory = server.directory / name
        self.directory.mkdir()
        self.tools = {}

    def __exit__(self, *_):
        pass

    def call(self, method, params=None):
        self.seq += 1
        message = rpc(method, params, self.seq)
        write(self.directory / f'{self.seq:03}.request.json', message)
        status, headers, data = self.server.request(message)
        write(self.directory / f'{self.seq:03}.http.json', dict(status=status, headers=headers))
        (self.directory / f'{self.seq:03}.response.body').write_bytes(data)
        assert headers['cache-control'] == 'no-store' and 'mcp-session-id' not in headers
        response = decode(headers, data)
        assert response['id'] == self.seq, response
        assert status == 200 if 'result' in response else status in (400, 404), (status, response)
        if method in ('tools/list', 'resources/list', 'resources/templates/list', 'resources/read') and 'result' in response:
            assert response['result']['ttlMs'] == 0 and response['result']['cacheScope'] == 'private'
        return response
