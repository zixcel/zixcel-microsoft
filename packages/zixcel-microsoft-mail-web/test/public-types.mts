import {MicrosoftMailSource,type MicrosoftMailMessage} from '@zixcel/microsoft-mail-web';
import {microsoftRedirectDocument} from '@zixcel/microsoft-mail-web/redirect-document';
const document: string=microsoftRedirectDocument();
const source=new MicrosoftMailSource('00000000-0000-4000-8000-000000000001');
const result: Promise<MicrosoftMailMessage[]>=source.list();
void result;void document;
