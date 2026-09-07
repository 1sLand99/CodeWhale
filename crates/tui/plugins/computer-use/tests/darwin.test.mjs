import { test } from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { spawn, spawnSync } from 'node:child_process';
import { create } from '../src/backends/darwin.mjs';
import { withSignal, currentSignal, runInputLease } from '../src/exec.mjs';

// Simulate the native lease process: it owns cleanup itself, keeping the
// original app identity even if the JS backend is rebound before release.
function leaseExecutor(run) {
  return { async run(cmd,args,opts) {
    const result=await run(cmd,args,opts);
    if(args[0]?.startsWith('{') && JSON.parse(args[0]).tool==='input_capabilities' && result.code===0 && JSON.parse(result.stdout).input_lease===undefined) return {code:0,stderr:'',stdout:JSON.stringify({input_lease:1})};
    return result;
  }, async runInputLease(cmd, argv) {
    const request = JSON.parse(argv[0]);
    const release = async ({point} = {}) => {
      const args = request.args;
      const native = request.tool === 'key_event'
        ? {tool:'key_event',args:{...args,down:false,owned_release:true}}
        : {tool:'release_input',args:{...args,point:point??args.steps.at(-1),button:0}};
      await withSignal(null,()=>run(cmd,[JSON.stringify(native)],{timeoutMs:20000,ownerPipe:true}));
    };
    const result = await run(cmd,argv,{timeoutMs:20000,ownerPipe:true});
    if (result.code !== 0 || result.aborted || result.timedOut) {
      if(result.spawned && (result.aborted || result.timedOut)) await release();
      throw Object.assign(new Error(result.aborted?'computer request cancelled':result.timedOut?'native accessibility helper timed out':result.stderr),{code:result.aborted?'cancelled':'native_error'});
    }
    return {receipt:JSON.parse(result.stdout),release,async send({point}) {
      const result=await run(cmd,[JSON.stringify({tool:'pointer_sequence',args:{...request.args,steps:[{type:6,...point,button:0}]}})],{timeoutMs:20000,ownerPipe:true});
      return JSON.parse(result.stdout);
    }};
  }};
}

test('native summary keeps text and top-level menus without spending the UI budget on hidden menu trees', {skip:process.platform!=='darwin'}, t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'cu-native-observation-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  const binary=path.join(dir,'native');
  const build=spawnSync('clang',['-DCU_TEST=1','-fobjc-arc','-Os','-framework','Cocoa','-framework','ApplicationServices','-framework','ScreenCaptureKit','-framework','AVFoundation','-framework','CoreMedia','-framework','Vision','src/backends/darwin-accessibility.m','-o',binary],{encoding:'utf8'});
  assert.equal(build.status,0,build.stderr);
  const node=(role,label,children=[])=>({AXRole:role,AXTitle:label,AXChildren:children,actions:['AXPress']});
  const app={AXMenuBar:node('AXMenuBar','Menu bar',[node('AXMenuBarItem','File',[node('AXMenu','File menu',Array.from({length:150},(_,i)=>node('AXMenuItem',`Command ${i}`)))])]),AXChildren:[node('AXMenu','Popup',[node('AXMenuItem','Choose')])]};
  const windows=[node('AXWindow','Fixture',Array.from({length:350},(_,i)=>({...node('AXTextField',`Field ${i}`),AXValue:`Value ${i}`,AXFocused:i===349})))];
  const observe=(detail)=>{
    const r=spawnSync(binary,[JSON.stringify({tool:'inspect_observation',args:{app,windows,detail}})],{encoding:'utf8'});
    assert.equal(r.status,0,r.stderr);return JSON.parse(r.stdout);
  };
  for(const detail of [undefined,'summary','compact']) {
    const state=observe(detail);
    assert.equal(state.truncated,false,'intentional menu summarization is not a truncated observation');
    assert.equal(state.elements.length,355);
    assert.ok(state.elements.some(e=>e.label==='File'));
    assert.ok(state.elements.some(e=>e.label==='Choose'),'open popup actions remain observable');
    assert.ok(!state.elements.some(e=>e.label==='Command 0'));
    const field=state.elements.find(e=>e.label==='Field 349');
    assert.equal(field.value,'Value 349');assert.equal(field.focused,true);
    assert.deepEqual(field.path,[349]);assert.equal(field.windowIndex,0);
  }
  const full=observe('full');
  assert.equal(full.truncated,false);
  assert.equal(full.elements.find(e=>e.label==='Command 149').windowIndex,-1);
  assert.deepEqual(full.elements.find(e=>e.label==='Command 149').path,[0,0,149]);
  assert.ok(full.elements.some(e=>e.label==='Field 349'));
});

test('native Unicode encoding round-trips through the actual CoreGraphics event', {skip:process.platform!=='darwin'}, t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'cu-native-test-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  const binary=path.join(dir,'native');
  const build=spawnSync('clang',['-DCU_TEST=1','-fobjc-arc','-Os','-framework','Cocoa','-framework','ApplicationServices','-framework','ScreenCaptureKit','-framework','AVFoundation','-framework','CoreMedia','-framework','Vision','src/backends/darwin-accessibility.m','-o',binary],{encoding:'utf8'});
  assert.equal(build.status,0,build.stderr);
  for(const text of ['Hello 世界 🐋','quote " slash \\ newline\n','e\u0301 👨‍👩‍👧‍👦']){
    const r=spawnSync(binary,[JSON.stringify({tool:'inspect_text_event',args:{text}})],{encoding:'utf8'});
    assert.equal(r.status,0,r.stderr);assert.equal(JSON.parse(r.stdout).text,text);
    t.diagnostic(`Constructed literal Unicode event flags: ${JSON.parse(r.stdout).flags}`);
  }
});

test('native window matching refuses another process, mismatched geometry and ambiguous captures', {skip:process.platform!=='darwin'}, t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'cu-native-window-'));t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  const binary=path.join(dir,'native');
  const build=spawnSync('clang',['-DCU_TEST=1','-fobjc-arc','-Os','-framework','Cocoa','-framework','ApplicationServices','-framework','ScreenCaptureKit','-framework','AVFoundation','-framework','CoreMedia','-framework','Vision','src/backends/darwin-accessibility.m','-o',binary],{encoding:'utf8'});
  assert.equal(build.status,0,build.stderr);
  const selected={kCGWindowOwnerPID:123,kCGWindowLayer:0,kCGWindowNumber:42,kCGWindowBounds:{X:10,Y:20,Width:400,Height:300}};
  const query=windows=>spawnSync(binary,[JSON.stringify({tool:'inspect_window_match',args:{pid:123,bounds:{x:10,y:20,w:400,h:300},windows}})],{encoding:'utf8'});
  const wrongProcess={...selected,kCGWindowOwnerPID:999,kCGWindowNumber:43};
  const wrongSize={...selected,kCGWindowNumber:44,kCGWindowBounds:{...selected.kCGWindowBounds,Width:800}};
  const found=query([wrongProcess,wrongSize,selected]);
  assert.equal(found.status,0,found.stderr);assert.equal(JSON.parse(found.stdout).window_id,42);
  const missing=query([wrongProcess,wrongSize]);
  assert.equal(missing.status,1);assert.match(missing.stderr,/selected app window is not capturable/);
  const ambiguous=query([selected,{...selected,kCGWindowNumber:45}]);
  assert.equal(ambiguous.status,1);assert.match(ambiguous.stderr,/selected window is ambiguous/);
});

test('native owner pipe survives forced MCP exit, releases promptly and excludes competing input', {skip:process.platform!=='darwin'}, async t=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'cu-native-owner-')); t.after(()=>fs.rmSync(dir,{recursive:true,force:true}));
  const binary=path.join(dir,'native');
  const build=spawnSync('clang',['-DCU_TEST=1','-fobjc-arc','-Os','-framework','Cocoa','-framework','ApplicationServices','-framework','ScreenCaptureKit','-framework','AVFoundation','-framework','CoreMedia','-framework','Vision','src/backends/darwin-accessibility.m','-o',binary],{encoding:'utf8'});
  assert.equal(build.status,0,build.stderr);
  for(const workMs of [0,5000]) {
  const releaseFile=path.join(dir,`released-${workMs}`);
  const args={owner_pipe:true,input_lease:true,lock_dir:dir,release_file:releaseFile,work_ms:workMs};
  const request=JSON.stringify({tool:'test_input_lease',args});
  const parent=spawn(process.execPath,['--input-type=module','-e',`
    import {runInputLease} from ${JSON.stringify(new URL('../src/exec.mjs',import.meta.url).href)};
    await runInputLease(process.argv[1],[process.argv[2]]);
    console.log('ready'); setInterval(()=>{},1000);
  `,binary,request],{stdio:['ignore','pipe','pipe']});
  t.after(()=>{if(parent.exitCode===null)parent.kill('SIGKILL');});
  await new Promise((resolve,reject)=>{parent.stdout.once('data',resolve);parent.once('exit',code=>reject(new Error(`parent exited before ready: ${code}`)));});
  const competing=spawnSync(binary,[JSON.stringify({tool:'test_input_lease',args:{lock_dir:dir}})],{encoding:'utf8'});
  assert.equal(competing.status,1);
  assert.match(competing.stderr,/another Computer Use session owns held input/);
  assert.ok(!fs.existsSync(releaseFile));
  const exited=new Promise(resolve=>parent.once('exit',resolve));parent.kill('SIGKILL');await exited;
  const deadline=Date.now()+2000;
  while((!fs.existsSync(releaseFile)||fs.readFileSync(releaseFile,'utf8')!=='released')&&Date.now()<deadline)await new Promise(resolve=>setTimeout(resolve,20));
  assert.equal(fs.readFileSync(releaseFile,'utf8'),'released','native cleanup ran on owner pipe EOF');
  const successor=await runInputLease(binary,[JSON.stringify({tool:'test_input_lease',args:{...args,work_ms:0}})]);await successor.release();
  }
});
test('macOS backend binds native input to the opened process and reports denied permissions honestly', async t=>{
  const bundle=fs.mkdtempSync(path.join(os.tmpdir(),'cu-bundle-test-'));const old=process.env.CODEWHALE_CU_APP_BUNDLE;
  t.after(()=>{if(old===undefined)delete process.env.CODEWHALE_CU_APP_BUNDLE;else process.env.CODEWHALE_CU_APP_BUNDLE=old;fs.rmSync(bundle,{recursive:true,force:true});});
  fs.mkdirSync(path.join(bundle,'Contents','MacOS'),{recursive:true});fs.writeFileSync(path.join(bundle,'Contents','MacOS','accessibility'),'');process.env.CODEWHALE_CU_APP_BUNDLE=bundle;
  const calls=[];
  const backend=create({exec:leaseExecutor(async (cmd,args,opts)=>{
    assert.ok(Number.isFinite(opts.timeoutMs));
    if(cmd==='open')return {code:0,stdout:'',stderr:''};
    if(cmd==='screencapture')return {code:1,stdout:'',stderr:'denied'};
    const request=JSON.parse(args[0]);calls.push(request);
    return {code:0,stderr:'',stdout:JSON.stringify(request.tool==='app_info'?{found:true,pid:123,bundle_id:'test.app'}:request.tool==='permissions'?{trusted:false}:{action_sent:true})};
  })});
  await backend.open_application({name:'TextEdit',activate:false});await backend.key({text:'cmd+n'});await backend.type({text:'Hello 世界 🐋'});
  const events=calls.filter(c=>c.tool==='key_event');assert.equal(events.length,2);assert.equal(events[0].args.code,45);assert.equal(events[0].args.flags,1<<20);assert.equal(events[0].args.input_app_ref.pid,123);assert.equal(events[1].args.down,false);
  assert.equal(calls.find(c=>c.tool==='type').args.text,'Hello 世界 🐋');
  const probe=await backend.probe();assert.equal(probe.permissions.accessibility,'denied');assert.equal(probe.capabilities.raw_input,false);assert.equal(probe.capabilities.screenshot,false);
});

test('macOS background binding avoids reopen and releases at the agent pointer, not the user pointer', async t=>{
  const bundle=fs.mkdtempSync(path.join(os.tmpdir(),'cu-quiet-test-'));const old=process.env.CODEWHALE_CU_APP_BUNDLE;
  t.after(()=>{if(old===undefined)delete process.env.CODEWHALE_CU_APP_BUNDLE;else process.env.CODEWHALE_CU_APP_BUNDLE=old;fs.rmSync(bundle,{recursive:true,force:true});});
  fs.mkdirSync(path.join(bundle,'Contents','MacOS'),{recursive:true});fs.writeFileSync(path.join(bundle,'Contents','MacOS','accessibility'),'');process.env.CODEWHALE_CU_APP_BUNDLE=bundle;
  const calls=[];
  const backend=create({exec:leaseExecutor(async (cmd,args)=>{
    assert.notEqual(cmd,'open','binding a running app must not reopen its windows');
    const request=JSON.parse(args[0]);calls.push(request);
    assert.notEqual(request.tool,'cursor_position','release must not sample the physical pointer');
    const body=request.tool==='app_info'?{found:true,pid:123,bundle_id:'test.app'}
      :request.tool==='window_at_point'?{found:true,owner_pid:123,owner_name:'TextEdit',window_id:9,layer:0}
      :{action_sent:true};
    return {code:0,stderr:'',stdout:JSON.stringify(body)};
  })});
  await backend.open_application({name:'TextEdit'});
  assert.equal(calls[0].args.activate,false);
  await assert.rejects(backend.left_mouse_up({}),/no agent pointer/);
  await backend.left_mouse_down({target:{x:100,y:200}});
  await backend.mouse_move({target:{x:140,y:250}});
  await backend.left_mouse_up({});
  const release=calls.at(-1);
  assert.equal(release.tool,'release_input');
  assert.deepEqual(release.args.point,{x:140,y:250},'release lands at the agent pointer');
  assert.equal(release.args.restore,false,'a held button is not put back');
  assert.equal(release.args.input_app_ref.pid,123);
  assert.ok(!calls.some(c=>c.tool==='preview_notify'),'background actions do not open preview');
});

/** Backend wired to a scripted native helper; returns the requests it made. */
function stubBackend(t, reply) {
  const bundle = fs.mkdtempSync(path.join(os.tmpdir(), 'cu-hit-test-'));
  const old = process.env.CODEWHALE_CU_APP_BUNDLE;
  t.after(() => { if (old === undefined) delete process.env.CODEWHALE_CU_APP_BUNDLE; else process.env.CODEWHALE_CU_APP_BUNDLE = old; fs.rmSync(bundle, { recursive: true, force: true }); });
  fs.mkdirSync(path.join(bundle, 'Contents', 'MacOS'), { recursive: true });
  fs.writeFileSync(path.join(bundle, 'Contents', 'MacOS', 'accessibility'), '');
  process.env.CODEWHALE_CU_APP_BUNDLE = bundle;
  const calls = [];
  const backend = create({ exec: leaseExecutor(async (cmd, args) => {
    const request = JSON.parse(args[0]);
    calls.push(request);
    const custom = reply(request);
    if (custom?.nativeResult) return custom.nativeResult;
    const body = request.tool === 'app_info' ? custom ?? { found: true, pid: 321, bundle_id: 'test.app' }
      : custom
      ?? (request.tool === 'window_at_point' ? { found: true, owner_pid: 321, owner_name: 'TextEdit', window_id: 9, layer: 0 }
        : { action_sent: true, restored: true });
    return { code: 0, stderr: '', stdout: JSON.stringify(body) };
  }) });
  return { backend, calls };
}

const PRESSABLE = { found: true, element: { role: 'AXButton', label: 'Tab B', actions: ['AXPress'] }, action: 'AXPress', action_sent: true };
const NOT_PRESSABLE = { found: false, reason: 'no_pressable_element' };

test('macOS refuses an old native helper before any held input is dispatched', async t => {
  const {backend,calls}=stubBackend(t,request=>request.tool==='input_capabilities'?{input_lease:0}:null);
  await backend.open_application({name:'Fixture'});
  await assert.rejects(backend.key({text:'cmd+n'}),/helper needs an update/);
  await assert.rejects(backend.left_mouse_down({target:{x:70,y:80}}),/helper needs an update/);
  assert.ok(!calls.some(request=>['key_event','pointer_sequence','release_input'].includes(request.tool)));
});

test('macOS pointer cleanup keeps its original app after a background rebind', async t => {
  const {backend,calls}=stubBackend(t,request=>request.tool==='app_info'?{found:true,pid:request.args.app_ref.name==='First'?321:654,bundle_id:'test.app'}:null);
  await backend.open_application({name:'First',activate:true});
  await backend.left_mouse_down({target:{x:70,y:80}});
  await backend.open_application({name:'Second',activate:false});
  await backend.releaseInput();
  const release=calls.find(request=>request.tool==='release_input');
  assert.equal(release.args.foreground_input,true,'release belongs to the native owner from the original binding');
  assert.equal(release.args.input_app_ref.pid,321);
});

test('macOS cancellation releases a held key without replaying it', async t => {
  const controller = new AbortController();
  const { backend, calls } = stubBackend(t, request => {
    if (request.tool === 'key_event' && request.args.down) controller.abort();
    if (request.tool === 'key_event' && !request.args.down) assert.equal(currentSignal(), null);
  });
  await backend.open_application({name:'Fixture'});
  await assert.rejects(withSignal(controller.signal, () => backend.hold_key({text:'shift+a', duration:30})), /cancelled/);
  assert.deepEqual(calls.filter(r=>r.tool==='key_event').map(r=>r.args.down), [true,false]);
});

test('macOS session cleanup releases only its owned mouse press once', async t => {
  const { backend, calls } = stubBackend(t, () => null);
  await backend.open_application({name:'Fixture'});
  await backend.releaseInput();
  assert.ok(!calls.some(r=>r.tool==='release_input'));
  await backend.left_mouse_down({target:{x:70,y:80}});
  await withSignal(AbortSignal.abort(), () => backend.releaseInput());
  await backend.releaseInput();
  const releases=calls.filter(r=>r.tool==='release_input');
  assert.equal(releases.length,1);
  assert.deepEqual(releases[0].args.point,{x:70,y:80});
});

test('macOS foreground delivery requires explicit activation and resets on background binding', async t => {
  const { backend, calls } = stubBackend(t, () => null);
  await backend.open_application({name:'Fixture',activate:true});
  assert.equal((await backend.key({text:'return'})).keyboard_delivery,'foreground-guarded');
  assert.ok(calls.filter(r=>r.tool==='key_event').every(r=>r.args.foreground_input));
  await backend.open_application({name:'Fixture',activate:false});
  assert.equal((await backend.key({text:'return'})).keyboard_delivery,'process');
  assert.equal(calls.at(-1).args.foreground_input,false);
});

test('macOS foreground refusal never sends an unowned global key-up', async t => {
  const { backend, calls } = stubBackend(t, request => request.tool === 'key_event' && request.args.down
    ? { nativeResult: { code: 1, spawned: true, stdout: '', stderr: 'foreground changed to Mail (pid 999); expected Fixture (pid 321)' } } : null);
  await backend.open_application({name:'Fixture',activate:true});
  await assert.rejects(backend.key({text:'cmd+n'}), /foreground changed to Mail \(pid 999\)/);
  await assert.rejects(backend.hold_key({text:'shift+a',duration:30}), /expected Fixture/);
  assert.deepEqual(calls.filter(r=>r.tool==='key_event').map(r=>r.args.down), [true,true]);
});

test('macOS cancellation before child spawn does not release keys it never pressed', async t => {
  const { backend, calls } = stubBackend(t, request => request.tool === 'key_event' && request.args.down
    ? { nativeResult: { code: -1, spawned: false, aborted: true, stdout: '', stderr: '' } } : null);
  await backend.open_application({name:'Fixture',activate:true});
  await assert.rejects(backend.hold_key({text:'shift+a',duration:30}), /cancelled/);
  assert.deepEqual(calls.filter(r=>r.tool==='key_event').map(r=>r.args.down), [true]);
});

for (const failure of ['aborted','timedOut']) test(`macOS ${failure} after child spawn releases an ambiguous key press`, async t => {
  const { backend, calls } = stubBackend(t, request => {
    if(request.tool==='key_event' && request.args.down) return { nativeResult: { code: null, spawned: true, [failure]: true, stdout: '', stderr: '' } };
    if(request.tool==='key_event' && !request.args.down) assert.equal(currentSignal(),null);
  });
  await backend.open_application({name:'Fixture',activate:true});
  await assert.rejects(backend.key({text:'cmd+n'}), /cancelled|timed out/);
  const events=calls.filter(r=>r.tool==='key_event');
  assert.deepEqual(events.map(r=>r.args.down), [true,false]);
  assert.equal(events[1].args.owned_release,true);
});

test('macOS refused mouse-down cannot acquire release ownership', async t => {
  const { backend, calls } = stubBackend(t, request => request.tool==='window_at_point'
    ? {found:true,owner_pid:999,owner_name:'Mail'} : null);
  await backend.open_application({name:'Fixture'});
  await assert.rejects(backend.left_mouse_down({target:{x:70,y:80}}), /owned by Mail/);
  await backend.releaseInput();
  assert.ok(!calls.some(r=>['pointer_sequence','release_input'].includes(r.tool)));
});

test('macOS cancellation during the ownership probe cannot acquire release ownership', async t => {
  const { backend, calls } = stubBackend(t, request => request.tool==='window_at_point'
    ? {nativeResult:{code:null,spawned:true,aborted:true,stdout:'',stderr:''}} : null);
  await backend.open_application({name:'Fixture'});
  await assert.rejects(backend.left_mouse_down({target:{x:70,y:80}}), /cancelled/);
  await backend.releaseInput();
  assert.ok(!calls.some(r=>['pointer_sequence','release_input'].includes(r.tool)));
});

test('macOS a single mouse-down ownership guard precedes the dispatch and ambiguous cleanup', async t => {
  const { backend, calls } = stubBackend(t, request => request.tool==='pointer_sequence'
    ? {nativeResult:{code:null,spawned:true,aborted:true,stdout:'',stderr:''}} : null);
  await backend.open_application({name:'Fixture'});
  await assert.rejects(backend.left_mouse_down({target:{x:70,y:80}}), /cancelled/);
  assert.equal(calls.filter(r=>r.tool==='window_at_point').length,1);
  await backend.releaseInput();
  assert.equal(calls.at(-1).args.point.x,70);
  assert.equal(calls.at(-1).args.point.y,80);
  assert.equal(calls.at(-1).tool,'release_input');
});

test('macOS coordinate left_click prefers the accessibility element under the point', async (t) => {
  const { backend, calls } = stubBackend(t, (r) => (r.tool === 'hit_test' ? PRESSABLE : null));
  await backend.open_application({ name: 'TextEdit' });
  const receipt = await backend.left_click({ target: { x: 220, y: 180 } });
  assert.equal(receipt.strategy, 'a11y');
  assert.equal(receipt.action, 'AXPress');
  assert.equal(receipt.element.label, 'Tab B');
  const hit = calls.find((c) => c.tool === 'hit_test');
  assert.deepEqual([hit.args.x, hit.args.y, hit.args.perform], [220, 180, true]);
  assert.equal(hit.args.input_app_ref.pid, 321);
  assert.ok(!calls.some((c) => c.tool === 'mouse_event'), 'a semantic press must not also post raw pointer events');
});

test('macOS coordinate left_click falls back to a guarded global gesture when no element is pressable', async (t) => {
  const { backend, calls } = stubBackend(t, (r) => (r.tool === 'hit_test' ? NOT_PRESSABLE : null));
  await backend.open_application({ name: 'TextEdit' });
  const receipt = await backend.left_click({ target: { x: 40, y: 90 } });
  assert.equal(receipt.strategy, 'event');
  assert.equal(receipt.pointer_moved, true, 'the receipt admits the real cursor moved');
  assert.equal(receipt.a11y_reason, 'no_pressable_element');

  const guard = calls.find((c) => c.tool === 'window_at_point');
  assert.deepEqual([guard.args.x, guard.args.y], [40, 90], 'ownership of the landing point is checked first');
  const seq = calls.find((c) => c.tool === 'pointer_sequence');
  assert.deepEqual(seq.args.steps.map((s) => s.type), [5, 1, 2], 'move, down, up in one gesture');
  assert.deepEqual([seq.args.steps[1].x, seq.args.steps[1].y, seq.args.steps[1].clickState], [40, 90, 1]);
  assert.equal(seq.args.restore, true, 'the user gets their pointer back');
});

test('macOS refuses a global gesture whose landing point belongs to another application', async (t) => {
  const { backend, calls } = stubBackend(t, (r) => (r.tool === 'hit_test' ? NOT_PRESSABLE
    : r.tool === 'window_at_point' ? { found: true, owner_pid: 999, owner_name: 'Mail', window_id: 4, layer: 0 } : null));
  await backend.open_application({ name: 'TextEdit' });
  await assert.rejects(backend.left_click({ target: { x: 40, y: 90 } }), /covered by a window owned by Mail/);
  assert.ok(!calls.some((c) => c.tool === 'pointer_sequence'), 'nothing is posted into the other application');
});

test('macOS left_click strategies: event skips the tree, a11y fails closed, other clicks stay pointer-driven', async (t) => {
  const { backend, calls } = stubBackend(t, (r) => (r.tool === 'hit_test' ? NOT_PRESSABLE : null));
  await backend.open_application({ name: 'TextEdit' });

  const forced = await backend.left_click({ target: { x: 10, y: 20 }, strategy: 'event' });
  assert.equal(forced.strategy, 'event');
  assert.ok(!calls.some((c) => c.tool === 'hit_test'), 'strategy=event never hit-tests');

  await assert.rejects(backend.left_click({ target: { x: 10, y: 20 }, strategy: 'a11y' }), /no pressable accessibility element/);
  await assert.rejects(backend.left_click({ target: { x: 10, y: 20 }, strategy: 'sideways' }), /strategy must be auto, a11y or event/);

  calls.length = 0;
  const dbl = await backend.double_click({ target: { x: 10, y: 20 } });
  assert.equal(dbl.strategy, 'event');
  assert.deepEqual(calls.find((c) => c.tool === 'pointer_sequence').args.steps.map((s) => s.clickState), [0, 1, 1, 2, 2]);
  assert.equal((await backend.right_click({ target: { x: 10, y: 20 } })).strategy, 'event');
  assert.ok(!calls.some((c) => c.tool === 'hit_test'), 'only a left single click has an accessibility equivalent');
});

test('macOS drag and scroll travel as one gesture that puts the pointer back', async (t) => {
  const { backend, calls } = stubBackend(t, () => null);
  await backend.open_application({ name: 'TextEdit' });

  const drag = await backend.left_click_drag({ from_target: { x: 10, y: 10 }, to: { x: 110, y: 10 } });
  assert.equal(drag.pointer_moved, true);
  const dragSeq = calls.find((c) => c.tool === 'pointer_sequence');
  assert.equal(dragSeq.args.restore, true);
  assert.equal(dragSeq.args.steps.at(-1).type, 2, 'released at the destination');
  assert.deepEqual([dragSeq.args.steps.at(-1).x, dragSeq.args.steps.at(-1).y], [110, 10]);

  calls.length = 0;
  await backend.scroll({ target: { x: 10, y: 10 }, direction: 'down', amount: 3 });
  const scrollSeq = calls.find((c) => c.tool === 'pointer_sequence');
  const notches = scrollSeq.args.steps.filter((s) => s.scroll);
  assert.equal(notches.length, 3, 'one notch per unit of amount, like a real wheel');
  assert.deepEqual(notches[0].scroll, [0, -1]);
  assert.equal(scrollSeq.args.restore, true);
});

test('native hit_test fails closed without a bound application', { skip: process.platform !== 'darwin' }, (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cu-hit-native-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const binary = path.join(dir, 'native');
  const build = spawnSync('clang', ['-DCU_TEST=1', '-fobjc-arc', '-Os', '-framework', 'Cocoa', '-framework', 'ApplicationServices', '-framework', 'ScreenCaptureKit', '-framework', 'AVFoundation', '-framework', 'CoreMedia', '-framework', 'Vision', 'src/backends/darwin-accessibility.m', '-o', binary], { encoding: 'utf8' });
  assert.equal(build.status, 0, build.stderr);
  const r = spawnSync(binary, [JSON.stringify({ tool: 'hit_test', args: { x: 1, y: 1, perform: true } })], { encoding: 'utf8' });
  assert.equal(r.status, 1);
  assert.match(r.stderr, /open_application first/);
});

test('native type verifies delivery against the focused control and fails closed on a non-text focus', { skip: process.platform !== 'darwin' }, (t) => {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'cu-type-native-'));
  t.after(() => fs.rmSync(dir, { recursive: true, force: true }));
  const binary = path.join(dir, 'native');
  const build = spawnSync('clang', ['-DCU_TEST=1', '-fobjc-arc', '-Os', '-framework', 'Cocoa', '-framework', 'ApplicationServices', '-framework', 'ScreenCaptureKit', '-framework', 'AVFoundation', '-framework', 'CoreMedia', '-framework', 'Vision', 'src/backends/darwin-accessibility.m', '-o', binary], { encoding: 'utf8' });
  assert.equal(build.status, 0, build.stderr);
  const type = (args) => spawnSync(binary, [JSON.stringify({ tool: 'inspect_type', args })], { encoding: 'utf8' });

  let r = type({ text: 'world', focused: { AXRole: 'AXTextField', AXValue: 'Hello ', after: 'Hello world' } });
  assert.equal(r.status, 0, r.stderr);
  let receipt = JSON.parse(r.stdout);
  assert.equal(receipt.action_sent, true);
  assert.equal(receipt.chars, 5);
  assert.equal(receipt.strategy, 'unicode-events');
  assert.equal(receipt.verified, true, 'suffix match confirms delivery');
  assert.equal(receipt.focused_role, 'AXTextField');
  assert.ok(!('verification_required' in receipt));

  r = type({ text: 'their', focused: { AXRole: 'AXTextArea', AXValue: 'I love ', after: 'I love thier' } });
  assert.equal(JSON.parse(r.stdout).verified, true, 'the length check survives an autocorrect transform');

  r = type({ text: 'x', focused: { AXRole: 'AXButton', AXTitle: 'Save' } });
  assert.equal(r.status, 1, 'a clearly non-text focus refuses before any event is posted');
  assert.match(r.stderr, /focused element is a AXButton, not a text control/);

  for (const focused of [null, { AXRole: 'AXWebArea' }, { AXRole: 'AXSecureTextField' }, { AXRole: 'AXTextField', AXValue: 'abc' }]) {
    r = type({ text: 'hi', focused });
    assert.equal(r.status, 0, r.stderr);
    receipt = JSON.parse(r.stdout);
    assert.equal(receipt.action_sent, true, 'unverifiable readbacks still report dispatch');
    assert.equal(receipt.verified, false);
    assert.equal(receipt.verification_required, 'screenshot');
  }
  assert.equal(JSON.parse(type({ text: 'hi', focused: null }).stdout).focused_role, null);
});

test('macOS type passes the native verification receipt through untouched', async (t) => {
  const nativeReceipt = { action_sent: true, chars: 5, strategy: 'unicode-events', keyboard_delivery: 'process', verified: false, focused_role: null, verification_required: 'screenshot' };
  const { backend } = stubBackend(t, (r) => (r.tool === 'type' ? nativeReceipt : null));
  await backend.open_application({ name: 'TextEdit' });
  assert.deepEqual(await backend.type({ text: 'hello' }), nativeReceipt);
});
