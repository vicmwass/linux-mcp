import hashlib
import json
from pathlib import Path
import shutil
import uuid
import threading
import unittest
from urllib.error import HTTPError
from urllib.request import urlopen
from http.server import ThreadingHTTPServer

from spec_status_server import snapshot, handler


class ViewerTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(__file__).resolve().parents[1] / 'tools/spec-status' / ('test-' + uuid.uuid4().hex)
        self.root.mkdir()
        self.addCleanup(shutil.rmtree, self.root)
        self.plans = self.root / 'specs/plans'
        self.store = self.plans / '.coord'
        self.store.mkdir(parents=True)
        self.registry = {'plan_id': '00-contracts', 'version': 1, 'steps': [
            {'id': 'a', 'title': '<script>text only</script>', 'depends_on': [], 'resources': ['a']},
            {'id': 'b', 'title': 'Optional', 'depends_on': ['a'], 'resources': ['b'], 'opt_in': 'assistant'}]}
        raw = json.dumps(self.registry).encode()
        (self.plans / '00-contracts.steps.json').write_bytes(raw)
        (self.plans / 'catalog.json').write_text(json.dumps({'version': 1, 'plans': ['00-contracts.steps.json']}))
        self.state = {'version': 1, 'plans': {'00-contracts': {
            'registry_sha256': hashlib.sha256(raw).hexdigest(), 'steps': {
                'a': {'status': 'completed', 'resources': ['a'], 'token': 'NEVER_EXPOSE', 'evidence': 'Passed', 'history': []},
                'b': {'status': 'pending', 'resources': ['b'], 'history': []}}}}}
        self.write_state()

    def write_state(self):
        tmp = self.store / 'next.json'
        tmp.write_text(json.dumps(self.state))
        tmp.replace(self.store / 'state.json')

    def read(self):
        return snapshot(self.root)

    def test_read_only_and_projection(self):
        before = {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in self.root.rglob('*') if p.is_file()}
        data = self.read()
        self.assertNotIn('NEVER_EXPOSE', json.dumps(data))
        self.assertNotIn('token', json.dumps(data))
        self.assertEqual(data['plans'][0]['status'], 'completed')
        self.assertEqual(data['plans'][0]['total'], 1)
        self.assertEqual(data['plans'][0]['optional_total'], 1)
        self.assertEqual(before, {p: (p.read_bytes(), p.stat().st_mtime_ns) for p in self.root.rglob('*') if p.is_file()})

    def test_reopen_atomic_replace_and_dependencies(self):
        self.read()
        record = self.state['plans']['00-contracts']['steps']['a']
        record.update(status='pending', attempt=2)
        self.write_state()
        plan = self.read()['plans'][0]
        self.assertEqual(plan['completed'], 0)
        self.assertEqual(plan['steps'][0]['attempt'], 2)
        self.assertIsNone(plan['steps'][0]['evidence'])
        self.assertEqual(plan['steps'][1]['waiting_for'], ['00-contracts/a'])

    def test_missing_corrupt_and_drift(self):
        path = self.store / 'state.json'
        path.unlink()
        with self.assertRaises(OSError): self.read()
        path.write_text('{')
        with self.assertRaises(ValueError): self.read()
        self.write_state()
        (self.plans / '00-contracts.steps.json').write_text(json.dumps(self.registry) + ' ')
        with self.assertRaises(ValueError): self.read()

    def test_heartbeat_and_tooling_reservation(self):
        a = self.state['plans']['00-contracts']['steps']['a']
        a.update(status='claimed', owner='worker', heartbeat_at='2020-01-01T00:00:00+00:00')
        self.write_state()
        step = self.read()['plans'][0]['steps'][0]
        self.assertTrue(step['stale_heartbeat'])
        self.assertEqual(step['owner'], 'worker')
        a['status'] = 'pending'
        self.state['plans']['tooling'] = {'steps': {'x': {'status': 'claimed', 'resources': ['a']}}}
        self.write_state()
        step = self.read()['plans'][0]['steps'][0]
        self.assertFalse(step['eligible'])
        self.assertEqual(step['resource_conflicts'], {'a': 'tooling/x'})

    def test_http_routes_and_unavailable(self):
        server = ThreadingHTTPServer(('127.0.0.1', 0), handler(self.root))
        thread = threading.Thread(target=server.serve_forever, daemon=True)
        thread.start()
        self.addCleanup(server.server_close)
        self.addCleanup(server.shutdown)
        base = f'http://127.0.0.1:{server.server_port}'
        with urlopen(base + '/') as response:
            page = response.read().decode()
            self.assertIn('textContent', page)
            self.assertNotIn('innerHTML', page)
            self.assertIn('setInterval(refresh,3000)', page)
        with urlopen(base + '/api/status') as response:
            self.assertEqual(response.headers['Cache-Control'], 'no-store')
            self.assertNotIn(b'NEVER_EXPOSE', response.read())
        with self.assertRaises(HTTPError) as error: urlopen(base + '/specs/plans/.coord/state.json')
        self.assertEqual(error.exception.code, 404)
        (self.store / 'state.json').write_text('invalid')
        with self.assertRaises(HTTPError) as error: urlopen(base + '/api/status')
        self.assertEqual(error.exception.code, 503)


if __name__ == '__main__':
    unittest.main()
