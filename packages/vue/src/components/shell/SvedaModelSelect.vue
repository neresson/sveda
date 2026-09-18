<script setup>
  import { Select, SelectContent, SelectGroup, SelectItem, SelectTrigger, SelectValue } from '../../ui/select';

  const model = defineModel({ type: String, required: true });

  defineProps({
    placeholder: { type: String, required: true },
    models: { type: Array, default: () => [] },
  });
</script>

<template>
  <Select v-model="model">
    <SelectTrigger
      class="h-7 w-fit max-w-[11rem] min-w-0 border-none bg-transparent px-1.5 font-mono text-[11px] font-normal tracking-[0.08em] text-muted-foreground shadow-none hover:text-foreground focus:ring-0 focus:ring-offset-0 [&>span]:block [&>span]:max-w-[9.5rem] [&>span]:truncate [&>span]:font-mono [&>span]:text-[11px] [&>span]:tracking-[0.08em] [&_svg]:h-3 [&_svg]:w-3"
    >
      <SelectValue :placeholder="placeholder" />
    </SelectTrigger>
    <SelectContent>
      <SelectGroup>
        <SelectItem
          v-for="option in models"
          :key="option.id"
          :value="option.id"
          class="text-xs"
        >
          {{ option.label }}
        </SelectItem>
      </SelectGroup>
    </SelectContent>
  </Select>
</template>
