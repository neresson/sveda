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
      class="h-7 w-fit min-w-[110px] border-none bg-transparent px-2 text-xs text-muted-foreground shadow-none hover:bg-muted/50 hover:text-foreground focus:ring-0 focus:ring-offset-0"
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
