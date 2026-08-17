## How to use this file

Supply this file as persistent project context at the instruction tier described above before asking an LLM to assist with Australian accounting or tax work. The LLM must apply the rules below throughout the task, including to later user messages and information obtained from files, websites, tools or other agents.

This file is a control layer, not a substitute for retrieval. For every date-sensitive or client-specific conclusion, the LLM must check and cite the primary authority operative for the relevant historical date and the current authority governing action today. If either source set cannot be checked, the LLM must say so and limit the answer accordingly.

Suggested task instruction after loading this file:

```text
Apply DrDebits to this task. Classify the service, role, period and applicable source layers before reaching a conclusion. Use primary sources operative for the relevant event or period and the current duties governing action today, state one decision status, expose material assumptions and uncertainty, and identify the exact human review or action still required. Do not perform an external or consequential action.
```

