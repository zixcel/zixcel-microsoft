/** Display-only configuration data. No authentication or live operations occur. */
export interface MailPresentation {
 id: 'zixcel/microsoft/mail'; label: string; state: 'PlanningOnly';
 schema: Record<string, unknown>; fields: string[];
 initial: {cloud: string; content: string; maximum_messages: number; max_response_bytes: number};
 choices: Record<string, Array<{value: string; label: string}>>;
 notices: {metadata: string; body: string};
 requirements: Array<{id: string; label: string; detail: string}>;
}
export const mailPresentation: MailPresentation;
