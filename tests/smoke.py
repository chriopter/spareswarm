"""End-to-end API, persistence and CLI checks; all fixtures are temporary."""
import json, os, pathlib, socket, sqlite3, subprocess, tempfile, time, urllib.request, urllib.error
root = pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    db = pathlib.Path(tmp) / 'test.db'
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    token = 'temporary-test-token-at-least-24-characters'
    env = dict(os.environ, DATABASE_PATH=str(db), LISTEN_ADDR=f'127.0.0.1:{port}', WEB_DIR=str(root/'web'), SPARESWARM_TOKEN=token, SPARESWARM_URL=base)
    proc = subprocess.Popen([root/'target/debug/server'], env=env, stdout=subprocess.DEVNULL)
    def call(path, method='GET', data=None, auth=True):
        req=urllib.request.Request(base+path, method=method, data=json.dumps(data).encode() if data is not None else None, headers={'Content-Type':'application/json', **({'Authorization':f'Bearer {token}'} if auth else {})})
        try:
            with urllib.request.urlopen(req) as r: return r.status, json.loads(r.read())
        except urllib.error.HTTPError as e: return e.code, e.read().decode()
    try:
        for _ in range(100):
            try:
                urllib.request.urlopen(base+'/health'); break
            except OSError: time.sleep(.05)
        assert call('/api/projects')[1] == []
        assert call('/api/tasks')[1] == []
        assert call('/api/tasks','POST',{},False)[0] in (401,422)
        # A fixture project exists only in this disposable test database.
        with sqlite3.connect(db) as c: c.execute("INSERT INTO projects VALUES('fixture','Fixture','https://example.com/repo','Test task')")
        payload={'project_id':'fixture','title':'Test','description':'Reproduce'}
        assert call('/api/tasks','POST',payload,False)[0] == 401
        assert call('/api/tasks','POST',{**payload,'project_id':'missing'})[0] == 400
        status, task=call('/api/tasks','POST',payload); assert status == 201
        tid=task['id']
        assert call('/api/tasks/'+tid,'PATCH',{'status':'invalid'})[0] == 400
        assert call('/api/tasks/'+tid,'PATCH',{'content':'test','link':'javascript:alert(1)'})[0] == 400
        assert call('/api/tasks/'+tid,'PATCH',{'status':'working','content':'Evidence','link':'https://example.com'})[0] == 200
        assert len(call('/api/tasks?project=fixture&status=working')[1]) == 1
        result=subprocess.check_output([root/'target/debug/spareswarm','update',tid,'--status','done','--text','Finished'],env=env)
        assert json.loads(result)['status']=='done'
        assert len(call('/api/tasks/'+tid)[1]['updates'])==2
        assert call('/api/tasks/missing')[0]==404
        assert json.loads(subprocess.check_output([root/'target/debug/spareswarm','projects'],env=env))[0]['id']=='fixture'
        assert urllib.request.urlopen(base+'/').status==200
        print('PASS: empty database, auth, validation, filters, updates, CLI, static site')
    finally:
        proc.terminate(); proc.wait(timeout=5)
    with sqlite3.connect(db) as c: assert c.execute('SELECT status FROM tasks').fetchone()[0]=='done'
    print('PASS: database persisted')
