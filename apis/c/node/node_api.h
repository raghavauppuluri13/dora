#ifndef DORA_NODE_API_H
#define DORA_NODE_API_H
#include <stddef.h>
#include <stdint.h>

void *init_dora_context_from_env();
void free_dora_context(void *dora_context);

void *dora_next_event(void *dora_context);
void free_dora_event(void *dora_event);

enum DoraEventType {
    DoraEventType_Stop,
    DoraEventType_Input,
    DoraEventType_InputClosed,
    DoraEventType_Error,
    DoraEventType_Unknown,
};
enum DoraEventType read_dora_event_type(void *dora_event);

void read_dora_input_id(void *dora_event, char **out_ptr, size_t *out_len);
void read_dora_input_data_u8(void *dora_event, uint8_t **out_ptr,
                             size_t *out_len);
void read_dora_input_data_i32(void *dora_event, int **out_ptr, size_t *out_len);
void read_dora_input_data_f32(void *dora_event, float **out_ptr,
                              size_t *out_len);
void read_dora_input_data_u64(void *dora_event, uint64_t **out_ptr,
                              size_t *out_len);

int dora_send_output_u8(void *dora_context, char *id_ptr, size_t id_len,
                        uint8_t *data_ptr, size_t data_len);
int dora_send_output_i32(void *dora_context, char *id_ptr, size_t id_len,
                         int *data_ptr, size_t data_len);
int dora_send_output_f32(void *dora_context, char *id_ptr, size_t id_len,
                         float *data_ptr, size_t data_len);
int dora_send_output_u64(void *dora_context, char *id_ptr, size_t id_len,
                         uint64_t *data_ptr, size_t data_len);

#endif
