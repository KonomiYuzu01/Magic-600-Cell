"""Negative control, never route evidence. Refuse every invocation."""


def execute(context):
    context.refuse("LG3:capability.not_granted", layer="control_closed")
