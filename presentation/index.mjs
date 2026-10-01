// Provider-owned display data. It declares no HAT, semantic meaning or credentials.
import schema from '../schemas/mail-request-v1.schema.json' with {type:'json'}
export const mailPresentation={
 id:'zixcel/microsoft/mail',label:'Microsoft 365',schema,
 state:'PlanningOnly',
 fields:['cloud','content','maximum_messages','max_response_bytes'],
 initial:{cloud:'public',content:'metadata',maximum_messages:25,max_response_bytes:1048576},
 choices:{cloud:[{value:'public',label:'Microsoft public cloud'},{value:'us-government',label:'US Government (L4)'}],content:[{value:'metadata',label:'Sender, subject and dates'},{value:'body',label:'Message body as well'}]},
 notices:{metadata:'Mail.ReadBasic consent is required. Message bodies are excluded.',body:'Separate Mail.Read consent is required. Sending, editing and deleting mail are excluded.'},
 requirements:[
  {id:'account',label:'Connected account',detail:'Microsoft Graph account authorization is not connected to Hatter yet.'},
  {id:'folder',label:'Mail folder',detail:'Folders will be listed after the selected account is authorized.'},
  {id:'intake',label:'Mail input HAT',detail:'A working mail input HAT must be installed before receiving messages.'},
  {id:'admission',label:'Enable connection',detail:'Saving and enabling a sensory binding through the Work owner is not available yet.'}
 ]
}
