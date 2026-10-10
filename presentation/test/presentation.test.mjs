import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mailPresentation} from '../index.mjs';
test('display data states its unconnected account and bounded read-only policy',()=>{
 assert.equal(mailPresentation.state,'PlanningOnly');
 assert.equal(mailPresentation.initial.maximum_messages,25);
 assert.equal(mailPresentation.initial.max_response_bytes,1048576);
 assert.ok(mailPresentation.requirements.some(x=>x.id==='account'));
 assert.ok(mailPresentation.notices.metadata.includes('Mail.ReadBasic'));
 assert.ok(!('accessToken' in mailPresentation.initial));
});
