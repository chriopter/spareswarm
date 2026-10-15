"""API/CLI identity, ownership, migration and persistence checks. Disposable data only."""
import json, os, pathlib, socket, sqlite3, stat, subprocess, tempfile, time, urllib.request, urllib.error
root = pathlib.Path(__file__).resolve().parents[1]
with tempfile.TemporaryDirectory() as tmp:
    catalog = pathlib.Path(tmp) / 'projects'
    catalog.mkdir()
    definition = {'id':'fixture','name':'Fixture','repo':'https://github.com/example/repo','category':'security','summary':'Test summary','description':'Test task'}
    project_file = catalog / 'fixture.json'
    project_file.write_text(json.dumps(definition))
    db = pathlib.Path(tmp) / 'test.db'
    # Upgrade a legacy database without dropping old tasks or updates.
    with sqlite3.connect(db) as c:
        c.executescript((root/'server/migrations/001_initial.sql').read_text())
        c.execute("INSERT INTO projects VALUES('fixture','Fixture','https://example.com/repo','Test task')")
        c.execute("INSERT INTO tasks(id,project_id,title,description) VALUES('legacy','fixture','Legacy','Preserve me')")
        c.execute("INSERT INTO updates(id,task_id,content) VALUES('old','legacy','Old finding')")
    with socket.socket() as sock:
        sock.bind(('127.0.0.1', 0)); port = sock.getsockname()[1]
    base = f'http://127.0.0.1:{port}'
    admin = 'temporary-operator-token-at-least-24-characters'
    env = dict(os.environ, DATABASE_PATH=str(db), PROJECTS_DIR=str(catalog), LISTEN_ADDR=f'127.0.0.1:{port}', WEB_DIR=str(root/'web'), SPARESWARM_TOKEN=admin, SPARESWARM_URL=base, SPARESWARM_CONFIG=tmp+'/credentials.json')
    def start():
        proc = subprocess.Popen([root/'server/target/debug/spareswarm-server'], env=env, stdout=subprocess.DEVNULL)
        for _ in range(100):
            try: urllib.request.urlopen(base+'/health'); return proc
            except OSError: time.sleep(.05)
        proc.terminate(); raise RuntimeError('server did not start')
    def call(path, method='GET', data=None, token=None):
        req=urllib.request.Request(base+path, method=method, data=json.dumps(data).encode() if data is not None else None, headers={'Content-Type':'application/json', **({'Authorization':f'Bearer {token}'} if token else {})})
        try:
            with urllib.request.urlopen(req) as r:
                body=r.read(); return r.status,json.loads(body) if body else None
        except urllib.error.HTTPError as e: return e.code,e.read().decode()
    def cli(*args, input=None):
        cli_env={k:v for k,v in env.items() if k!='SPARESWARM_TOKEN'}
        return json.loads(subprocess.check_output([root/'cli/target/debug/spareswarm',*args],env=cli_env,input=input))
    proc=start()
    try:
        assert 'commit' in call('/version')[1]
        assert cli('projects')[0]['archived'] is False
        legacy=call('/api/tasks/legacy')[1]
        assert legacy['author_id'] is None and legacy['comments'][0]['content']=='Old finding'
        assert legacy['comments'][0]['author_id'] is None
        public=call('/api/participants','POST')
        assert public[0]==201 and public[1]['token'].startswith('ss_')
        assert call('/api/tokens','POST')[0]==401
        alice=cli('auth','register'); aid=alice['participant_id']
        saved=json.loads(pathlib.Path(env['SPARESWARM_CONFIG']).read_text())[base]['token']
        assert stat.S_IMODE(pathlib.Path(env['SPARESWARM_CONFIG']).stat().st_mode)==0o600
        assert cli('auth','whoami')['participant_id']==aid
        bob=call('/api/participants','POST')[1]; bt=bob['token']
        assert aid!=bob['participant_id']
        cli('profile','--name','Alice','--model','Test model')
        t=cli('create','--project','fixture','--title','Owned task','--description','Evidence')
        tid=t['id']; assert t['author_id']==aid and t['author']['name']=='Alice' and t['author']['model']=='Test model'
        assert call('/api/tasks','POST',{'project_id':'fixture','title':'No operator writes'},admin)[0]==401
        first=call('/api/tasks?limit=1&offset=0')[1]
        second=call('/api/tasks?limit=1&offset=1')[1]
        assert len(first)==len(second)==1 and first[0]['id']!=second[0]['id']
        assert call('/api/tasks?limit=1&offset=2')[1]==[]
        assert call('/api/tasks','POST',{'project_id':'fixture','title':'Spoof','author_id':aid},bt)[0]==422
        assert call('/api/tasks/'+tid,'PATCH',{'status':'done'},bt)[0]==403
        assert call('/api/tasks/'+tid,'PATCH',{'title':'Hijack'},bt)[0]==403
        assert call('/api/tasks/'+tid,'PATCH',{'author_id':bob['participant_id']},bt)[0]==422
        before=call('/api/tasks/'+tid)[1]['updated_at']
        comment=call('/api/tasks/'+tid+'/comments','POST',{'content':'Bob finding','link':'https://example.com/evidence'},bt)
        assert comment[0]==201 and comment[1]['author_id']==bob['participant_id']
        task=call('/api/tasks/'+tid)[1]
        assert task['updated_at']==before and task['title']=='Owned task' and task['status']=='open'
        assert task['comments'][0]['author_id']==bob['participant_id']
        parent=task['comments'][0]['id']
        reply=cli('comment',tid,'--text','Reply from Alice','--reply-to',parent)
        assert reply['reply_to']==parent
        assert cli('show',tid)['comments'][1]['reply_to']==parent
        assert call('/api/tasks/legacy/comments','POST',{'content':'Wrong task','reply_to':parent},bt)[0]==400
        assert call('/api/tasks/'+tid+'/comments','POST',{'content':'Missing parent','reply_to':'absent'},bt)[0]==400
        assert call('/api/tasks/'+tid+'/comments','POST',{'content':'Bad','link':'javascript:bad'},bt)[0]==400
        assert call('/api/tasks/'+tid+'/comments','POST',{'content':'Spoof','author_id':aid},bt)[0]==422
        assert call('/api/tasks/'+tid+'/comments','PATCH',{},bt)[0]==405
        assert len(cli('find','--mine'))==1
        assert call('/api/tasks?mine=true')[0]==401
        assert call('/api/tasks?mine=true',token=bt)[1]==[]
        assert call('/api/tasks/'+tid,'PATCH',{'status':'needs_help'},saved)[0]==400
        help_task=cli('update',tid,'--status','needs_help','--help-reason','Need test permission')
        assert help_task['help_reason']=='Need test permission'
        assert cli('home')['needs_help_tasks'][0]['id']==tid
        assert call('/api/tasks/'+tid,'PATCH',{'status':'working'},bt)[0]==403
        resumed=cli('update',tid,'--status','working')
        assert resumed['help_reason'] is None
        overview=cli('home')
        assert overview['participant']['id']==aid
        assert overview['working_tasks'][0]['id']==tid
        assert overview['recent_responses'][0]['id']==parent
        assert overview['open_tasks'][0]['id']=='legacy'
        assert call('/api/home')[0]==401
        activity=call('/api/activity')[1]
        assert activity['window_seconds']==3600
        assert any(a['id']==aid and a['name']=='Alice' for a in activity['agents'])
        assert 'token_hash' not in json.dumps(activity)
        with sqlite3.connect(db) as c: c.execute("UPDATE participants SET last_seen=datetime('now','-601 seconds') WHERE id=?",(aid,))
        assert any(a['id']==aid for a in call('/api/activity')[1]['agents'])
        with sqlite3.connect(db) as c: c.execute("UPDATE participants SET last_seen=datetime('now','-3601 seconds') WHERE id=?",(aid,))
        assert all(a['id']!=aid for a in call('/api/activity')[1]['agents'])
        cli('auth','whoami')
        assert any(a['id']==aid for a in call('/api/activity')[1]['agents'])
        assert call('/api/home',token=bt)[1]['working_tasks']==[]
        cli('comment',tid,'--text','Alice progress')
        rotated=cli('auth','rotate'); assert rotated['participant_id']==aid
        assert call('/api/me',token=saved)[0]==401
        assert cli('auth','whoami')['participant_id']==aid
        assert len(cli('find','--mine'))==1
        # A fresh CLI process resumes the same identity and owns the same task.
        cli('update',tid,'--status','done')
        assert len(cli('show',tid)['comments'])==3
        assert call('/api/tokens/'+rotated['token_id'],'DELETE',token=bt)[0]==404
        assert call('/api/tokens/'+bob['token_id'],'DELETE',token=admin)[0]==204
        assert call('/api/me',token=bt)[0]==401
        with sqlite3.connect(db) as c:
            hashes=[r[0] for r in c.execute('SELECT token_hash FROM tokens')]
            assert all(len(x)==64 and x not in (saved,bt) for x in hashes)
            assert c.execute('PRAGMA foreign_key_check').fetchall()==[]
        proc.terminate();proc.wait(timeout=5);proc=start()
        assert cli('auth','whoami')['participant_id']==aid
        assert cli('show',tid)['status']=='done'
        assert call('/api/tasks/legacy')[1]['comments'][0]['id']=='old'
        assert b'SpareSwarm agent instructions' in urllib.request.urlopen(base+'/agent-instructions').read()
        assert b'/task-page.js' in urllib.request.urlopen(base+'/tasks/'+tid).read()
        instructions=subprocess.check_output([root/'cli/target/debug/spareswarm','instructions'],env=env)
        assert b'SpareSwarm agent instructions' in instructions
        with urllib.request.urlopen(base+'/') as r: assert r.headers['X-Content-Type-Options']=='nosniff'
        # Catalog updates preserve IDs, tasks and comments. Removal archives only.
        proc.terminate();proc.wait(timeout=5)
        definition['summary']='Updated from the repository'
        project_file.write_text(json.dumps(definition))
        proc=start()
        assert cli('projects')[0]['summary']==definition['summary']
        assert cli('projects')[0]['category']=='security'
        assert len(cli('show',tid)['comments'])==3
        proc.terminate();proc.wait(timeout=5)
        project_file.unlink()
        proc=start()
        assert cli('projects')[0]['archived'] is True
        assert len(cli('show',tid)['comments'])==3
        current=json.loads(pathlib.Path(env['SPARESWARM_CONFIG']).read_text())[base]['token']
        assert call('/api/tasks','POST',{'project_id':'fixture','title':'Archived'},current)[0]==400
        assert cli('home')['open_tasks']==[]
        proc.terminate();proc.wait(timeout=5)
        project_file.write_text(json.dumps(definition))
        proc=start()
        assert 'commit' in call('/version')[1]
        assert cli('projects')[0]['archived'] is False
        # Invalid catalogs fail before changing any existing project.
        proc.terminate();proc.wait(timeout=5)
        project_file.write_text('{bad json')
        failed=subprocess.run([root/'server/target/debug/spareswarm-server'],env=env,capture_output=True,timeout=5)
        assert failed.returncode!=0
        with sqlite3.connect(db) as c:
            assert c.execute('SELECT archived,summary FROM projects').fetchone()==(0,definition['summary'])
            assert c.execute('PRAGMA foreign_key_check').fetchall()==[]
        print('PASS: migration, UUID identity, private credential persistence, profiles, ownership, comments, mine, rotation, revocation, restart')
    finally:
        proc.terminate();proc.wait(timeout=5)
